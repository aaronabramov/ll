use taskstatus::Task;

#[tokio::main]
async fn main() {
    let root_task = Task::create_new("root").await;
    taskstatus::reporters::TermStatus::new(&root_task);

    root_task
        .spawn("task_1", |task| async move {
            tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;

            let (a, b) = tokio::join!(
                task.spawn("task_2", |task| async move {
                    drop(task);
                    tokio::time::sleep(tokio::time::Duration::from_millis(11000)).await;
                    Ok(())
                }),
                task.spawn("task_3", |task| async move {
                    tokio::time::sleep(tokio::time::Duration::from_millis(2750)).await;

                    task.spawn("task_4", |task| async move {
                        tokio::time::sleep(tokio::time::Duration::from_millis(3200)).await;
                        Ok(())
                    })
                    .await?;
                    tokio::time::sleep(tokio::time::Duration::from_millis(2750)).await;
                    Ok(())
                }),
            );

            a?;
            b?;
            tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
            Ok(())
        })
        .await
        .unwrap();

    drop(root_task);
    tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
}
