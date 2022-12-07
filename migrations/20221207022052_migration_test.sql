-- Add migration script here
CREATE TABLE test (
    id int NOT NULL PRIMARY KEY auto_increment,
    title varchar(64) NOT NULL,
    complex bool NOT NULL DEFAULT FALSE
);