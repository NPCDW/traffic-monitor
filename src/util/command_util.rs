use std::process::Stdio;
use std::sync::Arc;
use tokio::io::BufReader;
use tokio::process::Command;
use tokio::{sync::Mutex, io::AsyncBufReadExt};
use std::path::Path;
use tokio::sync::mpsc;
use tokio::sync::mpsc::Receiver;

#[cfg(windows)]
macro_rules! traceScriptWin {
    () => {
        r#"
Write-Output (@"
> $Env:UserName@$Env:ComputerName`:$((pwd).path)$ 
"@ + @'
{}
'@);
& {}; if (!$?) {{exit 1}}
"#
    };
}

#[cfg(windows)]
// [System.Console]::OutputEncoding=[System.Text.Encoding]::GetEncoding(65001);
macro_rules! setupScriptWin {
    () => {
        r#"
$ErrorActionPreference = 'Stop';
$OutputEncoding = [console]::InputEncoding = [console]::OutputEncoding = New-Object System.Text.UTF8Encoding
& cd {};
"#
    };
}

#[cfg(windows)]
fn build_command(workdir: String, commands: Vec<String>) -> Command {
    use base64::{engine::general_purpose::STANDARD, Engine as _};

    if !Path::new(&workdir).exists() {
        panic!("mihomo workdir {} not exist", workdir)
    }
    let mut merge = String::new();
    merge.push_str(&format!(setupScriptWin!(), workdir));
    for command in commands {
        merge.push_str(&format!(
            traceScriptWin!(),
            command.replace("\n", "\n>> "),
            command
        ));
    }
    let arg = format!(
        r#"[System.Text.Encoding]::UTF8.GetString([System.Convert]::FromBase64String("{}")) | iex"#,
        STANDARD.encode(merge.as_bytes())
    );
    let mut command = Command::new("pwsh");
    command
        .arg("-noprofile")
        .arg("-noninteractive")
        .arg("-command")
        .arg(arg)
        .current_dir(workdir);
    command
}

#[cfg(unix)]
macro_rules! traceScriptLinux {
    () => {
        r#"
echo -n "> $(whoami)@$(hostname):$(pwd)\$ "
cat << 'c623c61f-be9e-4e54-99c2-00218425de09'
{}
c623c61f-be9e-4e54-99c2-00218425de09
{}
"#
    };
}

#[cfg(unix)]
fn build_command(workdir: String, commands: Vec<String>) -> Command {
    if !Path::new(&workdir).exists() {
        panic!("mihomo workdir {} not exist", workdir)
    }
    let mut arg = String::new();
    for command in commands {
        arg.push_str(&format!(
            traceScriptLinux!(),
            command.replace("\n", "\n>> "),
            command
        ));
    }
    let mut command = Command::new("sh");
    command
        .arg("-c")
        .arg(arg)
        .current_dir(workdir);
    command
}

#[allow(dead_code)]
pub fn execute_to_spawn(
    workdir: String,
    commands: Vec<String>,
) -> Result<tokio::process::Child, std::io::Error> {
    let mut command = build_command(workdir, commands);
    command.spawn()
}

#[allow(dead_code)]
pub async fn execute_to_output(
    workdir: String,
    commands: Vec<String>,
) -> Result<std::process::Output, std::io::Error> {
    let mut command = build_command(workdir, commands);
    command.output().await
}

#[allow(dead_code)]
pub async fn execute_to_status(
    workdir: String,
    commands: Vec<String>,
) -> Result<std::process::ExitStatus, std::io::Error> {
    let mut command = build_command(workdir, commands);
    command.status().await
}

#[allow(dead_code)]
pub fn execute_async(workdir: String, commands: Vec<String>) -> (Option<u32>, Receiver<String>) {
    let mut command = build_command(workdir, commands);
    let spawn = command
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("failed to execute process");
    let pid = spawn.id();
    tracing::debug!("create subprocess {:?}", pid);

    let (tx, rx) = mpsc::channel(200);
    let tx_clone = tx.clone();
    let spawn = Arc::new(Mutex::new(spawn));
    let spawn_clone = spawn.clone();
    tokio::spawn(async move {
        let stdout = { spawn.lock().await.stdout.take() };
        if stdout.is_none() {
            return;
        }
        let mut stdout_lines = BufReader::new(stdout.unwrap()).lines();
        loop {
            if let Ok(Some(line)) = stdout_lines.next_line().await {
                if tx.send(line).await.is_err() {
                    break;
                }
            }
            if let Ok(Some(_status)) = { spawn.lock().await.try_wait() } {
                tracing::debug!("subprocess {:?} {:?} && stdout exit", pid, _status);
                break;
            }
        }
    });
    tokio::spawn(async move {
        let stderr = { spawn_clone.lock().await.stderr.take() };
        if stderr.is_none() {
            return;
        }
        let mut stderr_lines = BufReader::new(stderr.unwrap()).lines();
        loop {
            let res = stderr_lines.next_line().await;
            if let Ok(Some(line)) = &res {
                if tx_clone.send(line.clone()).await.is_err() {
                    break;
                }
            } else if let Err(e) = &res {
                tracing::error!("{}", e.to_string());
            }
            if let Ok(Some(_status)) = { spawn_clone.lock().await.try_wait() } {
                tracing::debug!("subprocess {:?} {:?} && stderr exit", pid, _status);
                break;
            }
        }
    });

    (pid, rx)
}

#[cfg(test)]
mod command_util_test {
    use std::time::Duration;

    use tokio::time::sleep;

    use crate::config;

    use super::*;

    #[tokio::test]
    async fn execute_to_status_test() {
        let config = config::app_config::get_config();
        config::log::init(&config);

        let commands = Vec::from([
            "echo $env:path".to_string(),
            "echo '123'".to_string(),
            r#"echo "456""#.to_string(),
            r#"Write-Output (@'
``$ `' `" '\''
'@);"#.to_string(),
r#"Write-Output (@"
``$ `' `" '\''
"@);"#.to_string(),
            "cd E:/Temp".to_string(),
            "dir".to_string(),
            "taskkill /IM mihomo.exe /T /F".to_string(),
        ]);
        let _ = execute_to_status("D:/Temp".to_string(), commands).await;

        sleep(Duration::from_secs(5)).await;
    }

    #[tokio::test]
    async fn execute_to_output_test() {
        let config = config::app_config::get_config();
        config::log::init(&config);

        let commands = Vec::from([
            "dir".to_string(),
        ]);
        let result = execute_to_output("C:/Project".to_string(), commands).await;
        if result.is_err() {
            let message = format!("查看日志失败，{}", result.err().unwrap().to_string());
            panic!("{}", message)
        }
        let output = result.unwrap();
        let mut message = String::from_utf8_lossy(&output.stdout).to_string();
        message.push_str(String::from_utf8_lossy(&output.stderr).to_string().as_str());

        println!("{}", message)
    }

    #[tokio::test]
    async fn execute_async_test() {
        let config = config::app_config::get_config();
        config::log::init(&config);

        let commands = Vec::from([
            format!("echo $'456{} {}\n23\n5'", r"\'", r#"\""#),
            // r".\mihomo.exe run -c config-4443.json".to_string(),
        ]);
        let (pid, mut rx) = execute_async(r"/data/wayfinder".to_string(), commands);
        // let (pid, mut rx) = execute_async(r"D:\Temp\proxy\mihomo".to_string(), commands);
        tracing::info!("{:?}", pid);
        loop {
            let res = rx.recv().await;
            if res.is_some() {
                tracing::info!("{}", res.unwrap());
            } else {
                break;
            }
        }
    }
}