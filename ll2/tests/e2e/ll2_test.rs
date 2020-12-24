// use anyhow::Result;
// use ll::ll2;

// #[tokio::test]
// async fn basic() -> Result<()> {
//     let l = ll2::create_logger()?;

//     l.event(|e| {
//         dbg!(e);
//         Ok(())
//     })?;

//     l.async_event(|e| async move {
//         dbg!(e);
//         l.event(|e| Ok(()))?;
//         dbg!(l);
//         Ok(())
//     })
//     .await?;

//     l.event(|_| Ok(()))?;

//     dbg!(l);

//     Ok(())
// }
