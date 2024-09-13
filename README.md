# traffic-monitor

## 简介

流量监控与控制

每15秒采集一次网卡的流量数据，并将数据保存到数据库中
- 开关机会丢失15秒的流量数据
- 重启应用不会丢失关闭应用期间的流量使用数据

配置流量周期后，会统计每个周期的流量使用情况，超过配置的流量限制后，会执行配置的超限命令

配置 `tg` 参数后，会发送每日的流量使用报告，如果同时配置了流量周期参数，也会发送流量使用过半，超80%，超90%，超过限制的通知

配置 `web` 参数后，会启动一个web服务，提供一个网页，用于查看流量使用情况

配置详情请查看 [conifg.example.json](./config/conifg.example.json)

## 使用

将 [docker-compose.yml](./docker-compose.yml) 放到任意服务器目录下，在同目录下创建 `config/config.json` 文件，配置请参考 [conifg.example.json](./config/conifg.example.json)，然后执行 `docker compose up -d` 即可

## 开发

`fork` 此项目，然后打开 `github codespace` 即可，更推荐使用 [MarsCode](https://www.marscode.com/dashboard) 进行开发

此项目的前端为 [traffic-monitor-web](https://github.com/NPCDW/traffic-monitor-web.git)
