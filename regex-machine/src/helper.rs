pub trait SafeAdd: Sized {
    fn safe_add(&self, n: &Self) -> Option<Self>;
}

impl SafeAdd for usize {
    fn safe_add(&self, n: &Self) -> Option<Self> {
        self.checked_add(*n)
    }
}

pub fn safe_add<T, F, E>(dst: &mut T, src: &T, f: F) -> Result<(), E>
where
    T: SafeAdd,
    F: Fn() -> E,
{
    if let Some(n) = dst.safe_add(src) {
        *dst = n;
        Ok(())
    } else {
        Err(f())
    }
}

// `Send`と`Sync`があるのでマルチスレッドで共有可能。かつ`static`ライフタイム境界なので、`static`でない参照を持たない
pub type DynError = Box<dyn std::error::Error + Send + Sync + 'static>;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_safe_add() {
        let n = 10;
        assert_eq!(n.safe_add(&20), Some(30));

        let n = !0;
        assert_eq!(n.safe_add(&1), None);

        let mut n = 10;
        assert!(safe_add(&mut n, &20, || ()).is_ok());

        let mut n = !0;
        assert!(safe_add(&mut n, &1, || ()).is_err());
    }
}
