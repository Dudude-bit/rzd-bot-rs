-- Your SQL goes here
CREATE TABLE tasks (
    id integer primary key autoincrement ,
    type text not null check ( type in ('train', 'day') ),
    data BLOB not null
);