use sea_orm::*;
use sea_orm_migration::prelude::*;
use super::migrator::Migrator;

pub async fn setup_db(request_url: &str, db_name: &str) -> Result<DatabaseConnection, DbErr>  {
    let db = Database::connect(request_url).await?;
    let db = match db.get_database_backend() {
       DbBackend::MySql => {
           db.execute(Statement::from_string(
               db.get_database_backend(),
               format!("CREATE DATABASE IF NOT EXISTS `{}`;", db_name),
           ))
           .await?;

           let url = format!("{}/{}", request_url, db_name);
           Database::connect(&url).await?
       }
       DbBackend::Postgres => {
           db.execute(Statement::from_string(
               db.get_database_backend(),
               format!("DROP DATABASE IF EXISTS \"{}\";", db_name),
           ))
           .await?;
           db.execute(Statement::from_string(
               db.get_database_backend(),
               format!("CREATE DATABASE \"{}\";", db_name),
           ))
           .await?;

           let url = format!("{}/{}", request_url, db_name);
           Database::connect(&url).await?
       }
       DbBackend::Sqlite => db,
    };

    let schema_manager = SchemaManager::new(&db); // To investigate the schema

    Migrator::up(&db.clone(), None).await?;
    assert!(schema_manager.has_table("clock_infos").await?);
    assert!(schema_manager.has_table("merge_logs").await?);
    assert!(schema_manager.has_table("z_messages").await?);
    assert!(schema_manager.has_table("bussiness_clocks").await?);

    Ok(db)
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::entities::{prelude::*, *};
    use futures::executor::block_on;

    const DATABASE_PG_URL: &str = "postgres://postgres:hetu@0.0.0.0:5432";
    const DB_NAME: &str = "vlc_inner_db";

    #[tokio::test]
    async fn set_up_db() {   // could add the function to server cli command
        match block_on(setup_db(DATABASE_PG_URL, DB_NAME)) {
            Err(err) => {
                panic!("{}", err);
            }
            Ok(_db) => async {  }.await,
        }
    }

    #[tokio::test]
    async fn test_run() {
        let url = format!("{}/{}", DATABASE_PG_URL, DB_NAME);
        let db = Database::connect(&url).await.expect("failed to connect to database");
        {
            let mut clock = vlc::Clock::new();
            clock.inc(0);
            clock.inc(1);
            let clock_str = serde_json::to_string(&clock).unwrap();
            let clock_info = clock_infos::ActiveModel {
                clock: ActiveValue::Set(clock_str.clone()),
                clock_hash: ActiveValue::Set("todo".to_owned()),
                node_id: ActiveValue::Set("todo".to_owned()),
                message_id: ActiveValue::Set("todo".to_owned()),
                raw_message: ActiveValue::Set(Vec::from("todo")),
                event_count: ActiveValue::Set(1),
                // create_at: ActiveValue::Set(current_time),
                ..Default::default()
            };
            let res = ClockInfos::insert(clock_info).exec(&db).await.expect("insert error");
            
            let clock_vec = ClockInfos::find().all(&db).await.expect("query error");
            println!("clock_vec-1 = {:?}", clock_vec);
            
            let clock_info2 = clock_infos::ActiveModel {
                id: ActiveValue::Set(res.last_insert_id),
                clock_hash: ActiveValue::Set("todo1".to_owned()),
                node_id: ActiveValue::Set("todo1".to_owned()),
                message_id: ActiveValue::Set("todo1".to_owned()),
                raw_message: ActiveValue::Set(Vec::from("todo1")),
                event_count: ActiveValue::Set(2),
                ..Default::default()
            };
            let _ = clock_info2.clone().update(&db).await;

            let mut clock3 = clock_info2;
            clock3.id = ActiveValue::Set(2);
            clock3.event_count = ActiveValue::Set(2);
            clock3.clock_hash = ActiveValue::Set("todo2".to_owned());
            clock3.clock = ActiveValue::Set(clock_str);
            println!("clock3 = {:?}", clock3);
            ClockInfos::insert(clock3).exec(&db).await.expect("insert error");
            let clock_vec = ClockInfos::find().all(&db).await.expect("query error");
            println!("clock_vec-2 = {:?}", clock_vec);
        }
    }

    #[tokio::test]
    async fn insert_zmessage() {
        let url = format!("{}/{}", DATABASE_PG_URL, DB_NAME);
        let db = Database::connect(&url).await.expect("failed to connect to database");
        {
            let zmessage = z_messages::ActiveModel {
                id: ActiveValue::Set(2),
                message_id: ActiveValue::Set("msg_id".to_string()),
                version: ActiveValue::Set(Some(1)),
                r#type: ActiveValue::Set(1),
                public_key: ActiveValue::Set(Some("pub_key_hex".to_owned())),
                data: ActiveValue::Set(Vec::from("zmessage.data")),
                signature: ActiveValue::Set(Some(Vec::new())),
                from: ActiveValue::Set("from_hex".to_owned()),
                to: ActiveValue::Set("to_hex".to_owned()),
                ..Default::default()
            };
            let res = ZMessages::insert(zmessage).exec(&db).await;
            if let Err(err) = res {
                println!("Insert z_messages error, err: {}", err);
            }
            let clock_vec = ZMessages::find().order_by_asc(z_messages::Column::Id).all(&db).await.expect("query error");
            println!("z_message-1 = {:?}", clock_vec);
        
        }
    }

    #[tokio::test]
    async fn test_get_clocks_by_msgid() {
        let url = format!("{}/{}", DATABASE_PG_URL, DB_NAME);
        let db = Database::connect(&url).await.expect("failed to connect to database");
        {
            let msg_id = "todo1";
            let clock_info = ClockInfos::find().filter(clock_infos::Column::MessageId.eq(msg_id)).all(&db).await.expect("query error");
            println!("pointed message_id's clocks = {:?}", clock_info);
        }
    }

    #[tokio::test]
    async fn test_get_clocks_from_start_id() {
        let url = format!("{}/{}", DATABASE_PG_URL, DB_NAME);
        let db = Database::connect(&url).await.expect("failed to connect to database");

        let start_id = 0;

        let clocks = ClockInfos::find()
            .filter(clock_infos::Column::Id.gt(start_id))
            .limit(1)
            .all(&db)
            .await
            .expect("query error");

        println!("clocks = {:?}", clocks);
    }

    #[tokio::test]
    async fn get_table_counts() {
        let url = format!("{}/{}", DATABASE_PG_URL, DB_NAME);
        let db = Database::connect(&url).await.expect("failed to connect to database");

        let clocks = ClockInfos::find()
            .count(&db)
            .await
            .expect("query error");

        println!("clocks = {:?}", clocks);
    }

    #[tokio::test]
    async fn get_last_clock() {
        let url = format!("{}/{}", DATABASE_PG_URL, DB_NAME);
        let db = Database::connect(&url).await.expect("failed to connect to database");

        let clocks = ClockInfos::find()
            .order_by_desc(clock_infos::Column::Id)
            .one(&db)
            .await
            .expect("query error");

        println!("clocks = {:?}", clocks);
    }
}