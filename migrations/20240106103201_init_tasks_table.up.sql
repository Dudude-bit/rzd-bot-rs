-- Add up migration script here
CREATE TABLE tasks (
    id integer primary key ,
    type text check ( type in ('day', 'train')),
    data jsonb
);