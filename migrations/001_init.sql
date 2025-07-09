
create table if not exists karbars (
    tid integer primary key not null,
    fullname text not null,
    username text,
    banned boolean not null default false,
    created_at integer not null,
    updated_at integer not null,
    points integer not null default 0,
    last_daily_point_at integer not null default 0
);

create table if not exists invite_links (
    link text primary key not null,
    karbar integer not null references karbars(tid) on delete cascade,
    count integer not null default 0
);

create table if not exists settings (
    id integer primary key not null,
    invite_points integer not null default 100,
    daily_points integer not null default 100
);

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
    proxy integer not null references proxies(id) on delete cascade
);

insert into settings(id) values(1);


/*
create table if not exists users (
    id integer primary key not null,
    name text,
    phone text not null,
    token text,
    photo text,
    --                           32 u8
    admin blob not null default x'0000000000000000000000000000000000000000000000000000000000000000',
    banned boolean not null default false
);

create table if not exists product_tags (
    id integer primary key not null,
    name text not null,
    kind integer not null,
    part integer not null,
    count integer not null default 0
);

create table if not exists products (
    id integer primary key not null,
    slug text not null unique,
    kind integer not null,
    name text not null,
    code text unique not null,
    detail text not null default "",
    created_at integer not null,
    updated_at integer not null default 0,
    thumbnail text,
    photos text not null default "[]",
    tag_leg integer references product_tags(id) on delete set null,
    tag_bed integer references product_tags(id) on delete set null,
    best boolean not null default false,
    description text not null default "",
    specification text not null default "{}",
    price integer not null default 0,
    count integer not null default 0
);

create table if not exists materials (
    id integer primary key not null,
    name text not null,
    detail text not null default "",
    created_at integer not null,
    updated_at integer not null default 0,
    updated_by integer references users(id) on delete set null,
    created_by integer references users(id) on delete set null,
    photo text,
    count integer not null default 0
);
*/
