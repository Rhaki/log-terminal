use std::{
    marker::PhantomData,
    ops::{Deref, DerefMut},
};

pub struct TypedVec<I: AsRef<usize>, T>(Vec<T>, PhantomData<I>);

impl<I, T> TypedVec<I, T>
where
    I: From<usize> + AsRef<usize>,
{
    pub fn new() -> Self {
        Self(Vec::new(), PhantomData)
    }

    pub fn from(vec: Vec<T>) -> Self {
        Self(vec, PhantomData)
    }

    pub fn get(&self, index: I) -> Option<&T> {
        self.0.get(*index.as_ref())
    }

    pub fn get_mut(&mut self, index: I) -> Option<&mut T> {
        self.0.get_mut(*index.as_ref())
    }

    pub fn remove(&mut self, index: I) -> T {
        self.0.remove(*index.as_ref())
    }

    pub fn len(&self) -> I {
        I::from(self.0.len())
    }
}

impl<I: AsRef<usize>, T> Deref for TypedVec<I, T> {
    type Target = Vec<T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<I: AsRef<usize>, T> DerefMut for TypedVec<I, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

macro_rules! generate_index {
    ($($ident:ident),*) => {
        $(
        #[derive(Default, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
        pub struct $ident(pub usize);

        impl AsRef<usize> for $ident {
            fn as_ref(&self) -> &usize {
                &self.0
            }
        }

        impl Deref for $ident {
            type Target = usize;
            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        impl From<usize> for $ident {
            fn from(value: usize) -> Self {
                Self(value)
            }
        }

        impl $ident {
            pub fn manipulate<R: Manipulable<Self>>(self, f: impl FnOnce(usize) -> R) -> R::Return {
                f(self.0).map()
            }
        }

        impl Manipulable<$ident> for usize {
            type Return = $ident;

            fn map(self) -> $ident {
                $ident(self)
            }
        }

        impl Manipulable<$ident> for Option<usize> {
            type Return = Option<$ident>;

            fn map(self) -> Option<$ident> {
                self.map(|index| $ident(index))
            }
        }

        impl<E> Manipulable<$ident> for Result<usize, E> {
            type Return = Result<$ident, E>;

            fn map(self) -> Result<$ident, E> {
                self.map(|index| $ident(index))
            }
        }
    )*
    };
}

generate_index!(PositionIdex, ContentIndex, TabIndex);

pub trait Manipulable<T> {
    type Return;
    fn map(self) -> Self::Return;
}
