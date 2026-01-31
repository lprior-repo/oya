use crate::Result;

pub type FallibleTransform<T, U> = fn(T) -> Result<U>;

pub type Validator<T> = fn(&T) -> Result<()>;

pub fn validate_all<T, F>(item: &T, validators: &[F]) -> Result<()>
where
    F: Fn(&T) -> Result<()>,
{
    validators
        .iter()
        .try_fold((), |(), validator| validator(item))
}

pub fn compose_result<T, U, V>(
    f: impl Fn(T) -> Result<U>,
    g: impl Fn(U) -> Result<V>,
) -> impl Fn(T) -> Result<V> {
    move |x| f(x).and_then(&g)
}

pub fn apply_transforms<T, F>(item: T, transforms: &[F]) -> Result<T>
where
    F: Fn(T) -> Result<T>,
{
    transforms
        .iter()
        .try_fold(item, |acc, transform| transform(acc))
}

pub fn group_by<T, K, F>(items: Vec<T>, key_fn: F) -> im::HashMap<K, Vec<T>>
where
    K: std::hash::Hash + Eq + Clone,
    T: Clone,
    F: Fn(&T) -> K,
{
    items.into_iter().fold(im::HashMap::new(), |mut map, item| {
        let key = key_fn(&item);
        let mut group = map.get(&key).cloned().unwrap_or_default();
        group.push(item);
        map.insert(key, group);
        map
    })
}

pub fn partition<T, F>(items: Vec<T>, predicate: F) -> (Vec<T>, Vec<T>)
where
    F: Fn(&T) -> bool,
{
    items.into_iter().partition(predicate)
}

pub fn fold_result<T, U, F>(items: Vec<T>, init: U, f: F) -> Result<U>
where
    F: Fn(U, T) -> Result<U>,
{
    items.into_iter().try_fold(init, f)
}

pub fn map_result<T, U, F>(items: Vec<T>, f: F) -> Result<Vec<U>>
where
    F: Fn(T) -> Result<U>,
{
    items.into_iter().map(f).collect()
}

pub fn filter_result<T, F>(items: Vec<T>, f: F) -> Result<Vec<T>>
where
    F: Fn(&T) -> Result<bool>,
{
    items.into_iter().try_fold(Vec::new(), |mut acc, item| {
        f(&item).map(|keep| {
            if keep {
                acc.push(item);
            }
            acc
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Error;

    fn is_positive(n: &i32) -> Result<()> {
        if *n > 0 {
            Ok(())
        } else {
            Err(Error::ValidationError("not positive".into()))
        }
    }

    fn is_even(n: &i32) -> Result<()> {
        if n % 2 == 0 {
            Ok(())
        } else {
            Err(Error::ValidationError("not even".into()))
        }
    }

    #[test]
    fn test_validate_all_success() {
        let validators: Vec<fn(&i32) -> Result<()>> = vec![is_positive, is_even];
        assert!(validate_all(&4, &validators).is_ok());
    }

    #[test]
    fn test_validate_all_failure() {
        let validators: Vec<fn(&i32) -> Result<()>> = vec![is_positive, is_even];
        assert!(validate_all(&3, &validators).is_err());
    }

    #[test]
    fn test_compose_result() {
        let double = |x: i32| -> Result<i32> { Ok(x * 2) };
        let add_one = |x: i32| -> Result<i32> { Ok(x + 1) };
        let composed = compose_result(double, add_one);

        assert_eq!(composed(5).unwrap_or_default(), 11);
    }

    #[test]
    fn test_group_by() {
        let items = vec![("a", 1), ("b", 2), ("a", 3), ("b", 4)];
        let grouped = group_by(items, |(key, _)| *key);

        assert_eq!(grouped.len(), 2);
        assert_eq!(grouped.get("a").map(Vec::len).unwrap_or(0), 2);
        assert_eq!(grouped.get("b").map(Vec::len).unwrap_or(0), 2);
    }

    #[test]
    fn test_partition() {
        let items = vec![1, 2, 3, 4, 5, 6];
        let (even, odd) = partition(items, |x| x % 2 == 0);

        assert_eq!(even, vec![2, 4, 6]);
        assert_eq!(odd, vec![1, 3, 5]);
    }

    #[test]
    fn test_fold_result() {
        let items = vec![1, 2, 3, 4, 5];
        let result = fold_result(items, 0, |acc, x| Ok(acc + x));
        assert_eq!(result.unwrap_or_default(), 15);
    }

    #[test]
    fn test_map_result() {
        let items = vec![1, 2, 3];
        let result = map_result(items, |x| Ok(x * 2));
        assert_eq!(result.unwrap_or_default(), vec![2, 4, 6]);
    }

    #[test]
    fn test_filter_result() {
        let items = vec![1, 2, 3, 4, 5];
        let result = filter_result(items, |x| Ok(x % 2 == 0));
        assert_eq!(result.unwrap_or_default(), vec![2, 4]);
    }
}
