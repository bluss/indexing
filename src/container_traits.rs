
/// The base container trait: The container can have indices and
/// ranges that are trusted to be in bounds.
pub unsafe trait Trustworthy {
    type Item;
    fn base_len(&self) -> usize;
}

/// The container has a contiguous addressable range.
pub unsafe trait Contiguous : Trustworthy {
    fn begin(&self) -> *const Self::Item;
    fn end(&self) -> *const Self::Item;
}

pub unsafe trait GetUnchecked : Trustworthy {
    unsafe fn xget_unchecked(&self, i: usize) -> &Self::Item;
}

pub unsafe trait GetUncheckedMut : GetUnchecked {
    unsafe fn xget_unchecked_mut(&mut self, i: usize) -> &mut Self::Item;
}

pub unsafe trait ContiguousMut : Contiguous {
    fn begin_mut(&self) -> *mut Self::Item {
        self.begin() as _
    }
    fn end_mut(&self) -> *mut Self::Item {
        self.end() as _
    }
}

unsafe impl<'a, C: ?Sized> Trustworthy for &'a C
    where C: Trustworthy
{
    type Item = C::Item;
    fn base_len(&self) -> usize {
        (**self).base_len()
    }
}

unsafe impl<'a, C: ?Sized> Trustworthy for &'a mut C
    where C: Trustworthy
{
    type Item = C::Item;
    fn base_len(&self) -> usize {
        (**self).base_len()
    }
}

unsafe impl<'a, C: ?Sized> ContiguousMut for &'a mut C
    where C: ContiguousMut
{ }

unsafe impl<'a, C: ?Sized> GetUnchecked for &'a C
    where C: GetUnchecked
{
    unsafe fn xget_unchecked(&self, i: usize) -> &Self::Item {
        (**self).xget_unchecked(i)
    }
}

unsafe impl<'a, C: ?Sized> GetUnchecked for &'a mut C
    where C: GetUnchecked
{
    unsafe fn xget_unchecked(&self, i: usize) -> &Self::Item {
        (**self).xget_unchecked(i)
    }
}

unsafe impl<'a, C: ?Sized> GetUncheckedMut for &'a mut C
    where C: GetUncheckedMut
{
    unsafe fn xget_unchecked_mut(&mut self, i: usize) -> &mut Self::Item {
        (**self).xget_unchecked_mut(i)
    }
}

unsafe impl<'a, C: ?Sized> Contiguous for &'a C
    where C: Contiguous,
{
    fn begin(&self) -> *const Self::Item {
        (**self).begin()
    }
    fn end(&self) -> *const Self::Item {
        (**self).end()
    }
}

unsafe impl<'a, C: ?Sized> Contiguous for &'a mut C
    where C: Contiguous,
{
    fn begin(&self) -> *const Self::Item {
        (**self).begin()
    }
    fn end(&self) -> *const Self::Item {
        (**self).end()
    }
}

unsafe impl<T> Trustworthy for [T] {
    type Item = T;
    fn base_len(&self) -> usize { self.len() }
}

unsafe impl<T> ContiguousMut for [T] { }

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

unsafe impl<T> Trustworthy for Vec<T> {
    type Item = T;
    fn base_len(&self) -> usize { self.len() }
}

unsafe impl<T> ContiguousMut for Vec<T> { }

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

pub unsafe trait Pushable : Trustworthy {
    fn push(&mut self, item: Self::Item) -> usize;
    unsafe fn insert_unchecked(&mut self, index: usize, item: Self::Item);
}

unsafe impl<T> Pushable for Vec<T> {
    fn push(&mut self, item: T) -> usize {
        let i = self.len();
        self.push(item);
        i
    }
    unsafe fn insert_unchecked(&mut self, index: usize, item: Self::Item) {
        self.insert(index, item)
    }
}

unsafe impl<'a, C: ?Sized> Pushable for &'a mut C
    where C: Pushable,
{
    fn push(&mut self, item: Self::Item) -> usize {
        (**self).push(item)
    }
    unsafe fn insert_unchecked(&mut self, index: usize, item: Self::Item) {
        (**self).insert_unchecked(index, item)
    }
}
