pub fn elapsed<T, E>(func: impl FnOnce() -> Result<T, E>) -> Result<(T, std::time::Duration), E> {
    let start = std::time::Instant::now();
    let result = func()?;
    let duration = start.elapsed();
    Ok((result, duration))
}
