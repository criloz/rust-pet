extern crate grpc;
extern crate futures;
extern crate tls_api;
extern crate tls_api_native_tls;
extern crate protobuf;
extern crate chrono;

#[macro_use]
extern crate diesel;
extern crate dotenv;

pub mod todo;
pub mod todo_grpc;
pub mod schema;
pub mod models;


use todo_grpc::{TaskManager, TaskManagerServer};
use std::thread;
use std::env;
use diesel::prelude::*;
use diesel::pg::PgConnection;
use dotenv::dotenv;
use chrono::prelude::*;

///try connect to the postgres db
pub fn establish_connection() -> Result<PgConnection, String> {
    dotenv().ok();
    let database_url = env::var("DATABASE_URL").map_err(|_| "DATABASE_URL must be set")?;
    PgConnection::establish(&database_url).map_err(|err| format!("Error connecting to {}, {}", database_url, err))
}

///read data from the task struct loaded from the db and put it in proto message
fn map_db_struct_to_proto(proto_task: &mut todo::Task, db_task: models::Task) {
    proto_task.set_name(db_task.name);
    let mut t = ::protobuf::well_known_types::Timestamp::new();
    t.set_seconds(db_task.created_at.timestamp());
    t.set_nanos(db_task.created_at.timestamp_subsec_nanos() as i32);
    proto_task.set_created_at(t);
    proto_task.set_id(db_task.id);
    proto_task.set_done(db_task.done);
    if let Some(done_time_db) = db_task.done_at {
        let mut done_time = ::protobuf::well_known_types::Timestamp::new();
        done_time.set_seconds(done_time_db.timestamp());
        done_time.set_nanos(done_time_db.timestamp_subsec_nanos() as i32);
        proto_task.set_done_at(done_time);
    }
}


struct TaskManagerImpl;

//implements task manager service
impl TaskManager for TaskManagerImpl {
    fn create(&self, _: grpc::RequestOptions, p: todo::TaskManagerCreateRequest) -> grpc::SingleResponse<todo::TaskManagerCreateResponse> {
        use models::{NewTask, Task as DBTask};
        use schema::tasks;
        use todo::{Task, Error};

        let mut r = todo::TaskManagerCreateResponse::new();

        //check if input contains a new _todo,  otherwise return error
        if !p.has_todo() {
            let mut error = Error::new();
            error.set_message("missing task definition".to_string());
            r.set_error(error);
            return grpc::SingleResponse::completed(r);
        }

        let todo = p.get_todo();

        //prepare insertion
        let new_task = NewTask { name: &todo.name, created_at: Utc::now().naive_utc() };

        match establish_connection() {
            Ok(conn) => {
                match diesel::insert_into(tasks::table).values(&new_task).get_result::<DBTask>(&conn) {
                    Ok(inserted) => {
                        let mut task = Task::new();
                        map_db_struct_to_proto(&mut task, inserted);
                        r.set_todo(task);
                    }
                    Err(err) => {
                        let mut error = Error::new();
                        error.set_message(format!("Task creation failed. {}", err));
                        r.set_error(error);
                    }
                }
            }
            Err(s) => {
                let mut error = Error::new();
                error.set_message(s);
                r.set_error(error);
            }
        }

        grpc::SingleResponse::completed(r)
    }

    fn update(&self, _: grpc::RequestOptions, p: todo::TaskManagerUpdateRequest) -> grpc::SingleResponse<todo::TaskManagerUpdateResponse> {
        use models::{UpdateTask, Task as DBTask};
        use schema::tasks::dsl::*;
        use todo::{Task, Error};

        let mut r = todo::TaskManagerUpdateResponse::new();

        //prepare update
        let mut update_task = UpdateTask { name: None, done_at: None, done: None };
        if p.has_done() {
            update_task.done = Some(p.get_done());
            if p.get_done() {
                update_task.done_at = Some(Some(Utc::now().naive_utc()))
            } else {
                update_task.done_at = Some(None)
            }
        }
        if p.has_name() {
            update_task.name = Some(p.get_name());
        }
        match establish_connection() {
            Ok(conn) => {
                match diesel::update(tasks.filter(id.eq(p.get_id()))).set(&update_task).get_result::<DBTask>(&conn) {
                    Ok(updated) => {
                        let mut task = Task::new();
                        map_db_struct_to_proto(&mut task, updated);
                        r.set_todo(task);
                    }
                    Err(err) => {
                        let mut error = Error::new();
                        error.set_message(format!("update task failed. {}", err));
                        r.set_error(error);
                    }
                }
            }
            Err(s) => {
                let mut error = Error::new();
                error.set_message(s);
                r.set_error(error);
            }
        }

        grpc::SingleResponse::completed(r)
    }

    fn delete(&self, _: grpc::RequestOptions, p: todo::TaskManagerDeleteRequest) -> grpc::SingleResponse<todo::TaskManagerDeleteResponse> {
        use schema::tasks::dsl::*;
        use todo::Error;
        let mut r = todo::TaskManagerDeleteResponse::new();
        match establish_connection() {
            Ok(conn) => {
                if let Err(err) = diesel::delete(tasks.filter(id.eq(p.get_id()))).execute(&conn) {
                    let mut error = Error::new();
                    error.set_message(format!("Task deletion failed. {}", err));
                    r.set_error(error)
                }
            }
            Err(s) => {
                let mut error = Error::new();
                error.set_message(s);
                r.set_error(error);
            }
        }
        grpc::SingleResponse::completed(r)
    }
    fn list(&self, _: grpc::RequestOptions, p: todo::TaskManagerListRequest) -> grpc::SingleResponse<todo::TaskManagerListResponse> {
        use models::Task as DBTask;
        use todo::{Task, Error};
        use schema::tasks::dsl::*;
        use protobuf::RepeatedField;

        let mut r = todo::TaskManagerListResponse::new();
        let mut task_list: RepeatedField<Task> = RepeatedField::new();

        match establish_connection() {
            Ok(conn) => {
                let mut result;
                match p.option {
                    todo::ListOptionKind::ALL => {
                        result = tasks.get_results::<DBTask>(&conn);
                    }
                    todo::ListOptionKind::DONE => {
                        result = tasks.filter(done.eq(true)).get_results::<DBTask>(&conn);
                    }
                    todo::ListOptionKind::NOT_DONE => {
                        result = tasks.filter(done.eq(false)).get_results::<DBTask>(&conn);
                    }
                }

                if let Ok(rows) = result {
                    for row in rows {
                        let mut task = Task::new();
                        map_db_struct_to_proto(&mut task, row);
                        task_list.push(task);
                    }
                }
            }

            Err(s) => {
                let mut error = Error::new();
                error.set_message(s);
                r.set_error(error);
            }
        }

        r.set_todo(task_list);
        grpc::SingleResponse::completed(r)
    }
}

fn main() {
    let port = 8080;

    let mut server: grpc::ServerBuilder<tls_api_native_tls::TlsAcceptor> = grpc::ServerBuilder::new();
    server.http.set_port(port);
    server.add_service(TaskManagerServer::new_service_def(TaskManagerImpl));
    server.http.set_cpu_pool_threads(4);
    let _server = server.build().expect("server");

    println!("server started on port {} {}", port, "without tls");

    loop {
        thread::park();
    }
}


#[cfg(test)]
mod tests {
    use super::TaskManagerImpl;
    use grpc;
    use super::todo;
    use super::todo_grpc::TaskManager;
    use super::establish_connection;
    use diesel;
    use diesel::prelude::*;

    enum CountKind {
        All,
        Done,
        NotDone,
    }

    fn clean_db() {
        use super::schema::tasks::dsl::*;
        //clean table
        let conn = establish_connection().unwrap();
        diesel::delete(tasks).execute(&conn).unwrap();
    }

    fn create_task(name: String) -> todo::TaskManagerCreateResponse {
        let server = TaskManagerImpl {};
        let ro = grpc::RequestOptions::new();
        let mut tr = todo::TaskManagerCreateRequest::new();
        let mut todo = todo::Task::new();
        todo.set_name(name);
        tr.set_todo(todo);
        server.create(ro, tr).wait_drop_metadata().unwrap()
    }

    fn delete_task(id: i32) -> todo::TaskManagerDeleteResponse {
        let server = TaskManagerImpl {};
        let ro = grpc::RequestOptions::new();
        let mut tr = todo::TaskManagerDeleteRequest::new();
        tr.set_id(id);
        server.delete(ro, tr).wait_drop_metadata().unwrap()
    }

    fn update_task(id: i32, optional_done: Option<bool>, optional_name: Option<String>) -> todo::TaskManagerUpdateResponse {
        let server = TaskManagerImpl {};
        let ro = grpc::RequestOptions::new();
        let mut tr = todo::TaskManagerUpdateRequest::new();
        if let Some(name) = optional_name {
            tr.set_name(name);
        }
        if let Some(done) = optional_done {
            tr.set_done(done);
        }
        tr.set_id(id);
        server.update(ro, tr).wait_drop_metadata().unwrap()
    }

    fn count_tasks(kind: CountKind) -> i64 {
        use super::schema::tasks::dsl::*;
        //clean table
        let conn = establish_connection().unwrap();
        match kind {
            CountKind::All => {
                return tasks.count().get_result::<i64>(&conn).unwrap();
            }
            CountKind::Done => {
                return tasks.filter(done.eq(true)).count().get_result::<i64>(&conn).unwrap();
            }
            CountKind::NotDone => {
                return tasks.filter(done.eq(false)).count().get_result::<i64>(&conn).unwrap();
            }
        }
    }

    fn list_tasks(kind: CountKind) -> todo::TaskManagerListResponse {
        let server = TaskManagerImpl {};
        let ro = grpc::RequestOptions::new();
        let mut tr = todo::TaskManagerListRequest::new();
        match kind {
            CountKind::All => {
                tr.set_option(todo::ListOptionKind::ALL)
            }
            CountKind::Done => {
                tr.set_option(todo::ListOptionKind::DONE)
            }
            CountKind::NotDone => {
                tr.set_option(todo::ListOptionKind::NOT_DONE)
            }
        }
        server.list(ro, tr).wait_drop_metadata().unwrap()
    }

    #[test]
    fn test_create_todo() {
        clean_db();
        let response = create_task("create grpc example".to_string());
        println!("{:?}", response);
        assert_eq!(response.get_todo().get_name(), "create grpc example");
    }

    #[test]
    fn test_delete_todo() {
        clean_db();
        let response = create_task("this task should be deleted".to_string());
        delete_task(response.get_todo().get_id());
        assert_eq!(count_tasks(CountKind::All), 0);
    }

    #[test]
    fn test_update_todo() {
        clean_db();
        let response = create_task("this task will be ser as done".to_string());
        let response = update_task(response.get_todo().get_id(), Some(true), None);
        assert_eq!(response.get_todo().get_done(), true);
        println!("{:?}", response);

        //now make it un done
        let response = update_task(response.get_todo().get_id(), Some(false), None);
        assert_eq!(response.get_todo().get_done(), false);
        println!("{:?}", response);
    }

    #[test]
    fn test_list_todos() {
        clean_db();
        let response_1 = create_task("Task1".to_string());
        let response_2 = create_task("Task2".to_string());
        let response_3 = create_task("Task2".to_string());
        let response_4 = create_task("Task4".to_string());
        let response_5 = create_task("Task5".to_string());

        update_task(response_1.get_todo().get_id(), Some(true), None);
        update_task(response_4.get_todo().get_id(), Some(true), None);
        update_task(response_5.get_todo().get_id(), Some(true), None);
        assert_eq!(count_tasks(CountKind::All), 5);
        assert_eq!(count_tasks(CountKind::Done), 3);
        assert_eq!(count_tasks(CountKind::NotDone), 2);
        assert_eq!(list_tasks(CountKind::All).get_todo().len(), 5);
        assert_eq!(list_tasks(CountKind::Done).get_todo().len(), 3);
        assert_eq!(list_tasks(CountKind::NotDone).get_todo().len(), 2);

    }
}

