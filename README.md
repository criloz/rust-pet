
A simple todo manager, that supports some basic crud command and allow to list all the tasks in the database also
filter them by their done status

## task model

| Column        | Type                  | Description  |
| ------------- |:--------------------: | -----:|
| name          | string                | Name of the task, it also can be use as description |
| created_at    | datetime              | time when the task is created  |
| done          | bool                  | when this is true the task is marked as completed |
| done_at       | Option<datetime>      | done_at will store the moment when the task  was marked as completed, if the task is set to done==False, this field is set to null |
 

## api design

the api was created around CRUD operation additionally some list query with basic filtering, 
in the implementation I took care and avoid panics,  instead an error was add to all the response message of
every rpc call 

## things to improve in the future

there is a transformation from `db response` to `diesel struct` to `protobuf struct`, there is room to  implement 
manually the diesel traits on the protobuf struct, and avoid unnecessary allocation

## important files

proto file: `src/todo.proto`
server implementation: `src/server.rs`
database models: `src/models.rs`

## important libraries

- `grpc-rust`
- `diesel` and rust orm


## testing

rust version: `rustc 1.27.0` (3eda71b00 2018-06-19) stable channel

- go to the project directory  in your teminal
- install diesel-cli: `cargo install diesel_cli --no-default-features --features "postgres"`
- add a postgres container with docker: `docker run -d -p 5432:5432 postgres`
- create database and tables: `diesel migration run`

run test with: `cargo test -- --test-threads=1`


## running

`cargo run --bin server`