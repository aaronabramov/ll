#[derive(PartialEq, Eq, PartialOrd, Ord, Debug, Clone, Copy)]
pub enum Level {
    Info = 1,
    Debug = 2,
    Trace = 3,
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn ordering() {
        let mut levels = vec![
            Level::Trace,
            Level::Debug,
            Level::Info,
            Level::Debug,
            Level::Trace,
        ];
        levels.sort();

        assert_eq!(
            levels,
            vec![
                Level::Info,
                Level::Debug,
                Level::Debug,
                Level::Trace,
                Level::Trace
            ]
        );
    }
}
