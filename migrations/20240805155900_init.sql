-- Add migration script here
create table monitor_second
(
    id          INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    create_time TIMESTAMP DEFAULT (datetime(CURRENT_TIMESTAMP,'localtime')) NOT NULL,
    start_time TIMESTAMP NOT NULL, -- 开始时间
    end_time TIMESTAMP NOT NULL, -- 结束时间
    uplink_traffic_readings int NOT NULL, -- 上行流量读数
    downlink_traffic_readings int NOT NULL, -- 下行流量读数
    uplink_traffic_usage int NOT NULL, -- 上行流量用量
    downlink_traffic_usage int NOT NULL, -- 下行流量用量
    time_interval int NOT NULL, -- 距上一次统计的时间间隔（秒）
    is_corrected int DEFAULT 0 NOT NULL -- 是否是修正数据
);

create table monitor_hour
(
    id          INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    create_time TIMESTAMP DEFAULT (datetime(CURRENT_TIMESTAMP,'localtime')) NOT NULL,
    day TIMESTAMP NOT NULL, -- 日期
    hour int NOT NULL, -- 小时，24小时制
    uplink_traffic_usage int NOT NULL, -- 上行流量用量
    downlink_traffic_usage int NOT NULL -- 下行流量用量
);

create table monitor_day
(
    id          INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    create_time TIMESTAMP DEFAULT (datetime(CURRENT_TIMESTAMP,'localtime')) NOT NULL,
    day TIMESTAMP NOT NULL, -- 日期
    uplink_traffic_usage int NOT NULL, -- 上行流量用量
    downlink_traffic_usage int NOT NULL -- 下行流量用量
);