pub fn wait(secs: u64) {
    println!("pausing for {}s...", secs);
    let begin = std::time::Instant::now();
    loop {
        let since = std::time::Instant::now().checked_duration_since(begin);
        if let Some(since) = since {
            if since.as_secs() >= secs {
                break;
            }
        }
    }
}