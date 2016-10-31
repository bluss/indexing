
pub unsafe trait Base {
    type Item;
    fn base_len(&self) -> usize;
}

pub unsafe trait Contiguous : Base {
    fn begin(&self) -> *const Self::Item;
    fn end(&self) -> *const Self::Item;
}

pub unsafe trait GetUnchecked : Base {
    unsafe fn xget_unchecked(&self, i: usize) -> &Self::Item;
}

pub unsafe trait GetUncheckedMut : GetUnchecked {
    unsafe fn xget_unchecked_mut(&mut self, i: usize) -> &mut Self::Item;
}

pub unsafe trait Mutable : Base { }

unsafe impl<'a, C> Base for &'a C
    where C: Base
{
    type Item = C::Item;
    fn base_len(&self) -> usize {
        (**self).base_len()
    }
}

unsafe impl<'a, C> Base for &'a mut C
    where C: Base
{
    type Item = C::Item;
    fn base_len(&self) -> usize {
        (**self).base_len()
    }
}

unsafe impl<'a, C> Mutable for &'a mut C
    where C: Mutable
{ }

unsafe impl<'a, C> GetUnchecked for &'a C
    where C: GetUnchecked
{
    unsafe fn xget_unchecked(&self, i: usize) -> &Self::Item {
        (**self).xget_unchecked(i)
    }
}

unsafe impl<'a, C> GetUnchecked for &'a mut C
    where C: GetUnchecked
{
    unsafe fn xget_unchecked(&self, i: usize) -> &Self::Item {
        (**self).xget_unchecked(i)
    }
}

unsafe impl<'a, C> GetUncheckedMut for &'a mut C
    where C: GetUncheckedMut
{
    unsafe fn xget_unchecked_mut(&mut self, i: usize) -> &mut Self::Item {
        (**self).xget_unchecked_mut(i)
    }
}

unsafe impl<'a, C> Contiguous for &'a C
    where C: Contiguous,
{
    fn begin(&self) -> *const Self::Item {
        (**self).begin()
    }
    fn end(&self) -> *const Self::Item {
        (**self).end()
    }
}

unsafe impl<'a, C> Contiguous for &'a mut C
    where C: Contiguous,
{
    fn begin(&self) -> *const Self::Item {
        (**self).begin()
    }
    fn end(&self) -> *const Self::Item {
        (**self).end()
    }
}

unsafe impl<T> Base for [T] {
    type Item = T;
    fn base_len(&self) -> usize { self.len() }
}

unsafe impl<T> Mutable for [T] { }

unsafe impl<T> GetUnchecked for [T] {
    unsafe fn xget_unchecked(&self, i: usize) -> &Self::Item {
        self.get_unchecked(i)
    }
}

unsafe impl<T> GetUncheckedMut for [T] {
    unsafe fn xget_unchecked_mut(&mut self, i: usize) -> &mut Self::Item {
        self.get_unchecked_mut(i)
    }
}

unsafe impl<T> Contiguous for [T] {
    fn begin(&self) -> *const Self::Item {
        self.as_ptr()
    }
    fn end(&self) -> *const Self::Item {
        unsafe {
            self.begin().offset(self.len() as isize)
        }
    }
}

unsafe impl<T> Base for Vec<T> {
    type Item = T;
    fn base_len(&self) -> usize { self.len() }
}

unsafe impl<T> Mutable for Vec<T> { }

unsafe impl<T> GetUnchecked for Vec<T> {
    unsafe fn xget_unchecked(&self, i: usize) -> &Self::Item {
        self.get_unchecked(i)
    }
}

unsafe impl<T> GetUncheckedMut for Vec<T> {
    unsafe fn xget_unchecked_mut(&mut self, i: usize) -> &mut Self::Item {
        self.get_unchecked_mut(i)
    }
}

unsafe impl<T> Contiguous for Vec<T> {
    fn begin(&self) -> *const Self::Item {
        (**self).begin()
    }

    fn end(&self) -> *const Self::Item {
        (**self).end()
    }
}

pub unsafe trait Pushable : Base {
    fn push(&mut self, item: Self::Item) -> usize;
}

unsafe impl<T> Pushable for Vec<T> {
    fn push(&mut self, item: T) -> usize {
        let i = self.len();
        self.push(item);
        i
    }
}

