pub(crate) trait Pwn {
    fn to_pwned(&self) -> Option<String>;
}

impl Pwn for Option<&str> {
    fn to_pwned(&self) -> Option<String> {
        self.as_ref().map(|s| s.to_string())
    }
}

// this does not work, GRRRRR
// pub(crate) trait Pwn<T> {
//     fn to_pwned(&self) -> Option<T>;
// }

// impl<T, U> Pwn<T> for Option<U>
// where
//     U: ToOwned<Owned = T>,
// {
//     fn to_pwned(&self) -> Option<T> {
//         self.as_ref().map(|inner| (*inner).to_owned())
//     }
// }

pub(crate) trait Unpwn {
    fn unpwn(&self) -> Option<&str>;
}

impl Unpwn for Option<&String> {
    fn unpwn(&self) -> Option<&str> {
        self.map(|s| s.as_str())
    }
}

impl Unpwn for Option<String> {
    fn unpwn(&self) -> Option<&str> {
        self.as_deref()
    }
}
