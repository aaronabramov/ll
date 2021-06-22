use taskstatus::println;
use taskstatus::Task;

#[tokio::main]
async fn main() {
    let root_task = Task::create_new("root").await;
    taskstatus::reporters::term_status::show().await;

    root_task
        .spawn("task_1", |task| async move {
            tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;

            let (a, b) = tokio::join!(
                task.spawn("task_2", |task| async move {
                    task.create("task_2.5").await;
                    tokio::time::sleep(tokio::time::Duration::from_millis(11000)).await;
                    Ok(())
                }),
                task.spawn("task_3", |task| async move {
                    tokio::time::sleep(tokio::time::Duration::from_millis(2750)).await;

                    println!(
                        "
                    print 
                    big amount of some random
                    output to stdout"
                    );

                    println!("hello");
                    println!("hey");

                    println!(
                        "and again
                     cause why not"
                    );
                    task.spawn("task_4", |_task| async move {
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
