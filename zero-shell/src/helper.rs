// `Send`と`Sync`があるのでマルチスレッドで共有可能。かつ`static`ライフタイム境界なので、`static`でない参照を持たない
pub type DynError = Box<dyn std::error::Error + Send + Sync + 'static>;