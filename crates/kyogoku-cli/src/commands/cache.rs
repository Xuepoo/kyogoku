use anyhow::Result;
use kyogoku_core::TranslationCache;

pub async fn stats() -> Result<()> {
    let cache = TranslationCache::open_default()?;

    println!("Cache Statistics:");
    println!("  Entries: {}", cache.len());

    Ok(())
}

pub async fn clear() -> Result<()> {
    let cache = TranslationCache::open_default()?;
    let count = cache.len();
    cache.clear()?;

    println!("✓ Cleared {} cached entries", count);

    Ok(())
}
