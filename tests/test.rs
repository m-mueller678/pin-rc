use pin_rc::PinRcStorage;
use std::pin::pin;

#[test]
fn single_rc() {
    let mut x = pin!(PinRcStorage::new(1));
    assert_eq!(x.ref_count(), 0);
    let h = x.as_ref().create_handle();
    assert!(x.as_mut().get_pin_mut().is_none());
    assert_eq!(h.ref_count(), 1);
    assert_eq!(*h, 1);
    drop(h);
    assert_eq!(*x.as_mut().get_pin_mut().unwrap(), 1);
}

#[test]
fn no_rc() {
    PinRcStorage::new(1);
}

#[test]
#[should_panic]
fn use_after_drop() {
    let _h = {
        let x = pin!(PinRcStorage::new(1));
        x.as_ref().create_handle()
    };
}
