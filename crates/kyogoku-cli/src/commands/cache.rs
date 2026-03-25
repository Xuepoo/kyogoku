use anyhow::Result;
use kyogoku_core::{TranslationCache, CacheStatus};

pub async fn stats() -> Result<()> {
    let cache = TranslationCache::open_default()?;
    let health = cache.health_check()?;

    println!("Cache Statistics:");
    println!("  Status:  {:?}", health.status);
    println!("  Entries: {}", health.entry_count);
    println!("  Size:    {}", health.disk_size_human());
    println!("  Path:    {}", cache.path().display());

    if health.corrupted_entries > 0 {
        println!("\n⚠️  Warning: {} corrupted entries detected", health.corrupted_entries);
        println!("   Run 'kyogoku cache clear' to reset the cache");
    }

    match health.status {
        CacheStatus::Healthy => println!("\n✓ Cache is healthy"),
        CacheStatus::Degraded => println!("\n⚠️  Cache is degraded but functional"),
        CacheStatus::Corrupted => println!("\n❌ Cache is corrupted and needs repair"),
    }

    Ok(())
}

pub async fn clear() -> Result<()> {
    let cache = TranslationCache::open_default()?;
    let count = cache.len();
    cache.clear()?;

    println!("✓ Cleared {} cached entries", count);

    Ok(())
}
