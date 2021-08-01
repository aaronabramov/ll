use ll::Task;
use std::sync::Arc;

#[tokio::main]
async fn main() {
    ll::add_reporter(Arc::new(ll::reporters::StdioReporter::new()));
    let root_task = Task::create_new("root #nostatus");
    ll::reporters::term_status::show().await;

    root_task
        .spawn("will_finish_fast", |task| async move {
            tokio::spawn(async move {
                tokio::time::sleep(tokio::time::Duration::from_millis(6000)).await;
                task.spawn("will_spawn_after_parent_is_done", |_| async move {
                    tokio::time::sleep(tokio::time::Duration::from_millis(10000)).await;

                    Ok(())
                })
                .await
                .ok();
            });
            Ok(())
        })
        .await
        .ok();

    Task::create_new("root2");
    root_task
        .spawn("task_1 #randomtag", |task| async move {
            tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
            let t_clone = task.clone();

            tokio::spawn(async move {
                t_clone
                    .spawn("detached_async_task", |_| async move {
                        tokio::time::sleep(tokio::time::Duration::from_millis(8400)).await;
                        Ok(())
                    })
                    .await
                    .ok();
            });

            let (a, b) = tokio::join!(
                task.spawn("task_2", |task| async move {
                    task.data("hey", 1);
                    task.data("yo", "sup");
                    task.data("dontprint #dontprint", 4);

                    task.create("task_2.5");
                    task.create("won't be printed #dontprint");

                    tokio::time::sleep(tokio::time::Duration::from_millis(11000)).await;
                    Ok(())
                }),
                task.spawn("task_3", |task| async move {
                    for i in 10..=99 {
                        let mut result = vec![];
                        let s = format!("{} {} <<<>>>", i, "-".repeat(i % 10));
                        result.push(s);
                        println!("{}", result.join("\n"));
                    }

                    tokio::time::sleep(tokio::time::Duration::from_millis(2750)).await;
                    task.data_transitive("transitive", 555);

                    task.spawn("task_4", |task| async move {
                        task.spawn("will_error", |task| async move {
                            task.spawn_sync("hello", |_task| Ok(()))?;
                            tokio::spawn(async move {
                                task.spawn("will run longer that parent", |_task| async move {
                                    tokio::time::sleep(tokio::time::Duration::from_millis(12000))
                                        .await;
                                    Ok(())
                                })
                                .await
                            });

                            anyhow::bail!("omg no i failed");
                            #[allow(unreachable_code)]
                            Ok(())
                        })
                        .await?;

                        tokio::time::sleep(tokio::time::Duration::from_millis(3200)).await;
                        Ok(())
                    })
                    .await?;
                    tokio::time::sleep(tokio::time::Duration::from_millis(2750)).await;
                    Ok(())
                }),
            );

            a.ok();
            b?;
            tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
            Ok(())
        })
        .await
        .map_err(|e| println!("{:?}", e))
        .ok();

    drop(root_task);
    tokio::time::sleep(tokio::time::Duration::from_millis(10000)).await;
}
