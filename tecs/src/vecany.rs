use std::any::TypeId;

pub struct VecAny {
    ptr: Option<*mut ()>,
    len: usize,
    cap: usize,
    ty: TypeId,
}

impl VecAny {
    pub fn new<T: 'static>() -> Self {
        Self::from_vec::<T>(Vec::new())
    }

    pub fn new_uninit(ty: TypeId) -> Self {
        Self {
            ptr: None,
            len: 0,
            cap: 0,
            ty,
        }
    }

    pub fn from_vec<T: 'static>(data: Vec<T>) -> Self {
        let (ptr, len, cap) = data.into_raw_parts();
        Self {
            ptr: Some(ptr.cast()),
            len,
            cap,
            ty: TypeId::of::<T>(),
        }
    }

    pub fn run<T: 'static>(&mut self, f: impl FnOnce(&mut Vec<T>)) {
        if self.ty != TypeId::of::<T>() {
            return;
        }

        let mut data: Vec<T> = match self.ptr {
            Some(ptr) => unsafe { Vec::<T>::from_raw_parts(ptr.cast(), self.len, self.cap) },
            None => Vec::<T>::new(),
        };
        f(&mut data);

        let (ptr, len, cap) = data.into_raw_parts();
        self.ptr = Some(ptr.cast());
        self.len = len;
        self.cap = cap;
    }

    pub fn downcast_ref<T: 'static>(&self) -> Option<&[T]> {
        if self.ty != TypeId::of::<T>() {
            return None;
        }

        Some(match self.ptr {
            Some(ptr) => unsafe { std::slice::from_raw_parts(ptr.cast(), self.len) },
            None => &[],
        })
    }

    pub fn downcast_mut<T: 'static>(&mut self) -> Option<&mut [T]> {
        if self.ty != TypeId::of::<T>() {
            return None;
        }

        Some(match self.ptr {
            Some(ptr) => unsafe { std::slice::from_raw_parts_mut(ptr.cast(), self.len) },
            None => &mut [],
        })
    }

    pub fn push<T: 'static>(&mut self, item: T) {
        self.run(|data| data.push(item))
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn ty(&self) -> TypeId {
        self.ty
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn test_normal() {
        let mut vecany = VecAny::new::<usize>();
        vecany.push(0_usize);
        vecany.push(1_usize);
        vecany.push(2_usize);
        let mut data = [0, 1, 2];
        assert_eq!(Some(data.as_slice()), vecany.downcast_ref::<usize>());
        assert_eq!(Some(data.as_mut_slice()), vecany.downcast_mut::<usize>());
    }

    #[test]
    pub fn test_uninit() {
        let mut vecany = VecAny::new_uninit(TypeId::of::<usize>());
        vecany.push(0_usize);
        vecany.push(1_usize);
        vecany.push(2_usize);
        let mut data = [0, 1, 2];
        assert_eq!(Some(data.as_slice()), vecany.downcast_ref::<usize>());
        assert_eq!(Some(data.as_mut_slice()), vecany.downcast_mut::<usize>());
    }
}
