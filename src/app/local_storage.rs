macro_rules! set {
    ($storage:expr, $key:expr, $value:expr) => {
        if let Some(storage) = ($storage).storage_mut() {
            storage.set_string($key, ($value).to_string())
        }
    };
}

macro_rules! get_num {
    ($storage:expr, $key:tt, $default:expr) => {
        if let Some(ref storage) = $storage {
            if let Some(data) = storage.get_string($key) {
                data.parse().unwrap_or($default)
            } else {
                $default
            }
        } else {
            $default
        }
    };
}

pub(crate) use get_num;
pub(crate) use set;
