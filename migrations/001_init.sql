
create table if not exists karbars (
    tid integer primary key not null,
    fullname text not null,
    username text,
    banned boolean not null default false,
    created_at integer not null,
    updated_at integer not null,
    points integer not null default 0,
    last_daily_point_at integer not null default 0,
    invite_code text not null unique,
    blocked boolean not null default false
);

create table if not exists settings (
    id integer primary key not null,
    invite_points integer not null default 100,
    daily_points integer not null default 100,
    proxy_cost integer not null default 100,
    v2ray_cost integer not null default 100,
    vip_cost integer not null default 200,
    vip_views integer not null default 0,
    vip_max_views integer not null default 100,
    vip_msg integer,
    donate_msg integer
);
insert into settings(id) values(1);

create table if not exists proxies (
    id integer primary key not null,
    server text not null,
    port text not null,
    secret text not null,
    up_votes integer not null default 0,
    dn_votes integer not null default 0,
    disabled boolean not null default false,
    unique (server, port, secret)
);

create table if not exists proxy_votes (
    id integer primary key not null,
    kind integer not null,
    karbar integer not null references karbars(tid) on delete cascade,
    proxy integer not null references proxies(id) on delete cascade,
    unique (karbar, proxy)
);

create table if not exists v2rays (
    id integer primary key not null,
    label text not null,
    link text not null unique,
    up_votes integer not null default 0,
    dn_votes integer not null default 0,
    disabled boolean not null default false
);

create table if not exists v2rays_votes (
    id integer primary key not null,
    kind integer not null,
    karbar integer not null references karbars(tid) on delete cascade,
    v2ray integer not null references v2rays(id) on delete cascade,
    unique (karbar, v2ray)
);

create table if not exists channels (
    id integer primary key not null,
    name text not null,
    amount integer not null default 0,
    max_sub integer not null default -1,
    enabled boolean not null default false
);

create table if not exists flyers (
    id integer primary key not null,
    label text not null,
    link text,
    mid integer not null,
    views integer not null default 0,
    max_views integer not null default -1,
    disabled boolean not null default false
);
