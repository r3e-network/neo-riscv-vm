// Rc兼容性测试：验证Rc是否与PolkaVM bump allocator兼容

extern crate alloc;
use alloc::rc::Rc;

#[test]
fn test_rc_basic() {
    let data = Rc::new(vec![1, 2, 3]);
    let clone1 = Rc::clone(&data);
    let clone2 = Rc::clone(&data);

    assert_eq!(Rc::strong_count(&data), 3);
    assert_eq!(*data, vec![1, 2, 3]);
    assert_eq!(*clone1, vec![1, 2, 3]);
    assert_eq!(*clone2, vec![1, 2, 3]);
}

#[test]
fn test_rc_nested() {
    let inner = Rc::new(vec![1, 2, 3]);
    let outer = Rc::new(vec![inner.clone(), inner.clone()]);

    assert_eq!(Rc::strong_count(&inner), 3);
    assert_eq!(outer.len(), 2);
}

#[test]
fn test_rc_drop() {
    let data = Rc::new(vec![1, 2, 3, 4, 5]);
    let clone1 = Rc::clone(&data);

    drop(clone1);
    assert_eq!(Rc::strong_count(&data), 1);

    drop(data);
    // 如果到这里没有崩溃，说明Rc的drop与allocator兼容
}
