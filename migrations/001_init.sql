
create table if not exists users (
    id integer primary key not null,
    fullname text not null,
    username text,
    banned boolean not null default false
);

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
