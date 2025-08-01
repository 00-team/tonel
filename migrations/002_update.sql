
alter table settings add column star_point_price integer not null default 2;
alter table settings add column free_point_delay integer not null default 43200;
alter table settings add column total_stars integer not null default 0;

alter table settings rename column daily_points to free_points;
alter table karbars rename column last_daily_point_at to last_free_point_at;
