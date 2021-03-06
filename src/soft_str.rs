use std::ops::{
    Index,  Range,
    RangeFrom, RangeTo,
    RangeFull,
};
use std::cmp::PartialEq;
use std::default::Default;
use std::fmt::{self, Display};
use std::borrow::{ToOwned, Cow};
use std::ffi::{OsString, OsStr};
use std::path::Path;
use std::net::{ToSocketAddrs, SocketAddr};
use std::str::{self, FromStr, EncodeUtf16};
use std::{vec, io};
use std::iter::{Iterator, DoubleEndedIterator};

// this import will become unused in future rust versions
// but won't be removed for now for supporting current
// rust versions
#[allow(warnings)]
use std::ascii::AsciiExt;

use error::FromSourceError;
use soft_char::SoftAsciiChar;
use soft_string::SoftAsciiString;

/// A `str` wrapper adding a "is us-ascii" soft constraint.
///
/// This means that it should be ascii but is not guaranteed to be
/// ascii. Which means non ascii chars _are not a safety issue_ just
/// a potential bug.
///
/// This is useful for situations where:
///   1. you would have many unsafe from str conversions/"unnecessary" checks with a
///      strict AsciiStr
///   2. you rarely have to strictly rely on the value being ascii
///
///
/// # Note
/// Some functions which should be implemented directly
/// on `SoftAsciiStr` like e.g. `trim_matches` are only
/// provided through `.as_str()`. This
/// is because the Pattern API and SliceIndex API is unstable
/// i.e. can only be implemented in unstable for now.
/// Once it gets stabilized (rust #27721/#35729) implementations
/// can be added
#[derive(Debug,  PartialEq, Eq, PartialOrd, Ord, Hash)]
// `repr(transparent)` ensures that the internal layout of
// `SoftAsciiStr` is same as that of `str`.
// Without this, `from_unchecked` and `from_unchecked_mut`
// are unsound.
#[repr(transparent)]
pub struct SoftAsciiStr(str);


impl SoftAsciiStr {

    #[inline(always)]
    pub fn from_unchecked(s: &str) -> &SoftAsciiStr {
        unsafe { &*( s as *const str as *const SoftAsciiStr) }
    }

    #[inline(always)]
    #[deprecated(since = "1.0.0", note="use from_unchecked")]
    pub fn from_str_unchecked(s: &str) -> &SoftAsciiStr {
        SoftAsciiStr::from_unchecked(s)
    }

    #[inline(always)]
    pub fn from_unchecked_mut(s: &mut str) -> &mut SoftAsciiStr {
        unsafe { &mut *( s as *mut str as *mut SoftAsciiStr) }
    }

    pub fn from_str(source: &str) -> Result<&Self, FromSourceError<&str>> {
        if source.is_ascii() {
            Ok(Self::from_unchecked(source))
        } else {
            Err(FromSourceError::new(source))
        }
    }

    /// reruns checks if the "is us-ascii" soft constraint is still valid
    pub fn revalidate_soft_constraint(&self) -> Result<&Self, FromSourceError<&str>> {
        if self.is_ascii() {
            Ok(self)
        } else {
            Err(FromSourceError::new(self.as_str()))
        }
    }

    #[inline(always)]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_soft_ascii_string(self: Box<SoftAsciiStr>) -> SoftAsciiString {
        //Box<SoftAsciiStr> -> Box<str> -> String -> SoftAsciiString
        //Safe: basicaly coerces Box<SoftAsciiStr> to Box<str>
        let as_str = Self::into_boxed_str(self);
        let string = str::into_string(as_str);
        SoftAsciiString::from_unchecked(string)
    }

    pub fn from_boxed_str(bs: Box<str>) -> Box<SoftAsciiStr> {
        unsafe { Box::from_raw(Box::into_raw(bs) as *mut SoftAsciiStr) }
    }

    #[inline]
    pub fn into_boxed_str(self: Box<SoftAsciiStr>) -> Box<str> {
        unsafe { Box::from_raw(Box::into_raw(self) as *mut str) }
    }

    #[inline]
    pub fn lines(&self) -> SoftAsciiLines {
        SoftAsciiLines::from(self)
    }

    #[inline]
    pub fn split_whitespace(&self) -> SoftAsciiSplitWhitespace {
        SoftAsciiSplitWhitespace::from(self)
    }

    #[inline]
    pub fn char_indices(&self) -> SoftAsciiCharIndices {
        SoftAsciiCharIndices::from(self)
    }

    #[inline]
    pub fn chars(&self) -> SoftAsciiChars {
        SoftAsciiChars::from(self)
    }

    pub fn split_at(&self, mid: usize) -> (&SoftAsciiStr, &SoftAsciiStr) {
        let (left, right) = self.as_str().split_at(mid);
        (SoftAsciiStr::from_unchecked(left), SoftAsciiStr::from_unchecked(right))
    }

    #[deprecated(since="1.1.0", note="deprecated in std")]
    pub unsafe fn slice_unchecked(&self, begin: usize, end: usize) -> &SoftAsciiStr {
        #[allow(deprecated)]
        SoftAsciiStr::from_unchecked(self.as_str().slice_unchecked(begin, end))
    }

    /// Proxy of [`std::str::get_unchecked`].
    ///
    /// Currently limited to the various range types:
    ///
    /// - `Range<usize>`
    /// - `RangeInclusive<usize>`
    /// - `RangeFrom<usize>`
    /// - `RangeTo<usize>`
    /// - `RangeToInclusive<usize>`
    /// - `RangeFull`
    ///
    /// Once all methods on `SliceIndex` are stable this
    /// can be implemented using `SliceIndex<SoftAsciiStr>`
    /// bounds.
    ///
    /// [`std::str::get_unchecked`]: https://doc.rust-lang.org/std/primitive.str.html#method.get_unchecked
    pub unsafe fn get_unchecked<I>(&self, index: I) -> &SoftAsciiStr
    where
        I: hidden::TempSliceIndexHelper
    {
        SoftAsciiStr::from_unchecked(self.as_str().get_unchecked::<I>(index))
    }



    /// returns a mutable `str` reference to the inner buffer
    ///
    /// # Soft Constraint
    /// be aware that it is very easy to introduce bugs when
    /// directly editing a `SoftAsciiStr` as an `str`. Still
    /// compared to a AsciiStr implementation this won't
    /// introduce unsafety, just possible brakeage of the
    /// soft constraint that the data should be ascii.
    pub fn inner_str_mut(&mut self) -> &mut str {
        &mut self.0
    }

    pub fn parse<F>(&self) -> Result<F, <F as FromStr>::Err>
         where F: FromStr
    {
        self.as_str().parse::<F>()
    }
}

mod hidden {
    use std::slice::SliceIndex;
    use std::ops::{Range, RangeFrom, RangeTo, RangeFull, RangeToInclusive, RangeInclusive};

    /// This is a workaround to be able to provide `get_unchecked` by now on stable.
    ///
    /// The problem is that we can't yet implement `SliceIndex<SoftAsciiStr>` as the
    /// methods of that trait are unstable. We also can't use `SliceIndex<SoftAsciiStr>`
    /// as where bound as this will lead to a braking change when we add support for
    /// `SliceIndex<SoftAsciiStr>` in the future.
    ///
    /// So instead we add this "helper" trait and prevent custom implementations of
    /// it by not re-exporting it.
    //NIT[rustc/sealed]: When/if rust provides a mechanism for sealed traits use that instead.
    pub trait TempSliceIndexHelper: SliceIndex<str, Output=str> {}

    impl TempSliceIndexHelper for Range<usize> {}
    impl TempSliceIndexHelper for RangeInclusive<usize> {}
    impl TempSliceIndexHelper for RangeFrom<usize> {}
    impl TempSliceIndexHelper for RangeTo<usize> {}
    impl TempSliceIndexHelper for RangeToInclusive<usize> {}
    impl TempSliceIndexHelper for RangeFull {}

}

//TODO FromStr with custom error

macro_rules! impl_wrap_returning_string {
    (pub > $(fn $name:ident(&self$(, $param:ident: $tp:ty)*)),*) => ($(
        impl SoftAsciiStr {
            #[inline]
            pub fn $name(&self $(, $param: $tp)*) -> SoftAsciiString {
                let as_str = self.as_str();
                let res = str::$name(as_str $(, $param)*);
                SoftAsciiString::from_unchecked(res)
            }
        }
    )*)
}

impl_wrap_returning_string!{
    pub >
    fn to_lowercase(&self),
    fn to_uppercase(&self),
    fn repeat(&self, n: usize)
}

macro_rules! impl_wrap_returning_str {
    (pub > $(
        $(#[$attr:meta])*
        fn $name:ident(&self$(, $param:ident: $tp:ty)*)
        $(#[$inner_attr:meta])*
    ),*) => (
        impl SoftAsciiStr {$(
            $(#[$attr])*
            #[inline]
            pub fn $name(&self $(, $param: $tp)*) -> &SoftAsciiStr {
                let as_str = self.as_str();
                $(#[$inner_attr])* {
                    let res = str::$name(as_str $(, $param)*);
                    SoftAsciiStr::from_unchecked(res)
                }
            }
        )*}
    );
}

impl_wrap_returning_str!{
    pub >
    #[deprecated(since="1.1.0", note="deprecated in std")]
    fn trim_right(&self) #[allow(deprecated)],
    #[deprecated(since="1.1.0", note="deprecated in std")]
    fn trim_left(&self) #[allow(deprecated)],
    fn trim_end(&self),
    fn trim_start(&self),
    fn trim(&self)
}

macro_rules! impl_wrapping {
    (pub > $(fn $name:ident(&self$(, $param:ident: $tp:ty)*) -> $ret:ty),*) => (
        impl SoftAsciiStr {$(
            #[inline]
            pub fn $name(&self $(, $param: $tp)*) -> $ret {
                str::$name(self.as_str() $(, $param)*)
            }
        )*}
    )
}

impl_wrapping! {
    pub >
    fn len(&self) -> usize,
    fn is_empty(&self) -> bool,
    fn is_char_boundary(&self, index: usize) -> bool,
    fn as_ptr(&self) -> *const u8,
    fn encode_utf16(&self) -> EncodeUtf16,
    fn is_ascii(&self) -> bool,
    fn as_bytes(&self) -> &[u8]
}

impl AsRef<SoftAsciiStr> for SoftAsciiStr {
    #[inline]
    fn as_ref(&self) -> &Self {
        self
    }
}
impl AsRef<str> for SoftAsciiStr {
    #[inline]
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl AsRef<[u8]> for SoftAsciiStr {
    #[inline]
    fn as_ref(&self) -> &[u8] {
        self.as_bytes()
    }
}

impl<'a> Default for &'a SoftAsciiStr {
    #[inline]
    fn default() -> &'a SoftAsciiStr {
        SoftAsciiStr::from_unchecked(Default::default())
    }
}

impl Display for SoftAsciiStr {
    fn fmt(&self, fter: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(fter)
    }
}

macro_rules! impl_index {
    ($($idx:ty),*) => ($(
        impl Index<$idx> for SoftAsciiStr {
            type Output = SoftAsciiStr;
            fn index(&self, index: $idx) -> &Self::Output {
                SoftAsciiStr::from_unchecked(self.0.index(index))
            }
        }
    )*);
}

impl_index! {
    Range<usize>,
    RangeFrom<usize>,
    RangeTo<usize>,
    RangeFull
}

impl ToOwned for SoftAsciiStr {
    type Owned = SoftAsciiString;

    fn to_owned(&self) -> SoftAsciiString {
        SoftAsciiString::from_unchecked(String::from(&self.0))
    }
}

impl PartialEq<SoftAsciiString> for SoftAsciiStr {
    fn eq(&self, other: &SoftAsciiString) -> bool {
        self == &**other
    }
}

impl<'a> PartialEq<SoftAsciiString> for &'a SoftAsciiStr {
    fn eq(&self, other: &SoftAsciiString) -> bool {
        *self == &**other
    }
}

impl PartialEq<SoftAsciiStr> for String {
    fn eq(&self, other: &SoftAsciiStr) -> bool {
        self.as_str() == other.as_str()
    }
}

impl PartialEq<String> for SoftAsciiStr {
    fn eq(&self, other: &String) -> bool {
        self.as_str() == other.as_str()
    }
}

impl<'a> PartialEq<&'a SoftAsciiStr> for String {
    fn eq(&self, other: &&'a SoftAsciiStr) -> bool {
        self.as_str() == other.as_str()
    }
}

impl<'a> PartialEq<String> for &'a SoftAsciiStr {
    fn eq(&self, other: &String) -> bool {
        self.as_str() == other.as_str()
    }
}

impl<'a> PartialEq<SoftAsciiStr> for str {
    fn eq(&self, other: &SoftAsciiStr) -> bool {
        self == other.as_str()
    }
}

impl PartialEq<str> for SoftAsciiStr {
    fn eq(&self, other: &str) -> bool {
        self.as_str() == other
    }
}

impl<'a> PartialEq<SoftAsciiStr> for Cow<'a, SoftAsciiStr> {
    fn eq(&self, other: &SoftAsciiStr) -> bool {
        self.as_str() == other.as_str()
    }
}

impl<'a> PartialEq<Cow<'a, SoftAsciiStr>> for SoftAsciiStr {
    fn eq(&self, other: &Cow<'a, SoftAsciiStr>) -> bool {
        self.as_str() == other.as_str()
    }
}

impl<'a, 'b> PartialEq<&'b SoftAsciiStr> for Cow<'a, SoftAsciiStr> {
    fn eq(&self, other: &&'b SoftAsciiStr) -> bool {
        self.as_str() == other.as_str()
    }
}

impl<'a, 'b> PartialEq<Cow<'a, SoftAsciiStr>> for &'a SoftAsciiStr {
    fn eq(&self, other: &Cow<'a, SoftAsciiStr>) -> bool {
        self.as_str() == other.as_str()
    }
}

impl<'a> PartialEq<SoftAsciiStr> for Cow<'a, str> {
    fn eq(&self, other: &SoftAsciiStr) -> bool {
        &*self == other.as_str()
    }
}

impl<'a> PartialEq<Cow<'a, str>> for SoftAsciiStr {
    fn eq(&self, other: &Cow<'a, str>) -> bool {
        self.as_str() == &*other
    }
}

impl<'a, 'b> PartialEq<&'b SoftAsciiStr> for Cow<'a, str> {
    fn eq(&self, other: &&'b SoftAsciiStr) -> bool {
        &*self == other.as_str()
    }
}

impl<'a, 'b> PartialEq<Cow<'b, str>> for &'a SoftAsciiStr {
    fn eq(&self, other: &Cow<'b, str>) -> bool {
        self.as_str() == &*other
    }
}

impl PartialEq<SoftAsciiStr> for OsString {
    fn eq(&self, other: &SoftAsciiStr) -> bool {
        other.as_str().eq(self)
    }
}

impl PartialEq<OsString> for SoftAsciiStr {
    fn eq(&self, other: &OsString) -> bool {
        self.as_str().eq(other)
    }
}

impl<'a> PartialEq<&'a SoftAsciiStr> for OsString {
    fn eq(&self, other: &&'a SoftAsciiStr) -> bool {
        other.as_str().eq(self)
    }
}

impl<'a> PartialEq<OsString> for &'a SoftAsciiStr {
    fn eq(&self, other: &OsString) -> bool {
        self.as_str().eq(other)
    }
}

impl PartialEq<SoftAsciiStr> for OsStr {
    fn eq(&self, other: &SoftAsciiStr) -> bool {
        other.as_str().eq(self)
    }
}
impl PartialEq<OsStr> for SoftAsciiStr {
    fn eq(&self, other: &OsStr) -> bool {
        self.as_str().eq(other)
    }
}

impl AsRef<OsStr> for SoftAsciiStr {
    fn as_ref(&self) -> &OsStr {
        self.as_str().as_ref()
    }
}

impl AsRef<Path> for SoftAsciiStr {
    fn as_ref(&self) -> &Path {
        self.as_str().as_ref()
    }
}

impl ToSocketAddrs for SoftAsciiStr {
    type Iter = vec::IntoIter<SocketAddr>;

    fn to_socket_addrs(&self) -> io::Result<vec::IntoIter<SocketAddr>> {
        self.as_str().to_socket_addrs()
    }
}

/// a wrapper around `Chars` turning each char into a `SoftAsciiChar`
///
/// This iterator is returned by `SoftAsciiChar::chars(&self)` instead
/// of `Chars`.
#[derive(Debug, Clone)]
pub struct SoftAsciiChars<'a> {
    inner: str::Chars<'a>
}

impl<'a> From<&'a SoftAsciiStr> for SoftAsciiChars<'a> {
    fn from(s: &'a SoftAsciiStr) -> SoftAsciiChars<'a> {
        SoftAsciiChars {
            inner: s.as_str().chars()
        }
    }
}

impl<'a> Iterator for SoftAsciiChars<'a> {
    type Item = SoftAsciiChar;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
            .map(SoftAsciiChar::from_unchecked)

    }

    #[inline]
    fn count(self) -> usize {
        self.inner.count()
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }

    fn last(self) -> Option<Self::Item> {
        self.inner.last()
            .map(SoftAsciiChar::from_unchecked)
    }
}

impl<'a> DoubleEndedIterator for SoftAsciiChars<'a> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.inner.next_back()
            .map(SoftAsciiChar::from_unchecked)
    }
}

/// a wrapper around `CharsIndices` turning each char into a `SoftAsciiChar`
///
/// This iterator is returned by `SoftAsciiChar::char_indices(&self)` instead
/// of `CharIndices`.
#[derive(Debug, Clone)]
pub struct SoftAsciiCharIndices<'a> {
    inner: str::CharIndices<'a>
}

impl<'a> From<&'a SoftAsciiStr> for SoftAsciiCharIndices<'a> {
    fn from(s: &'a SoftAsciiStr) -> SoftAsciiCharIndices<'a> {
        SoftAsciiCharIndices {
            inner: s.as_str().char_indices()
        }
    }
}

impl<'a> Iterator for SoftAsciiCharIndices<'a> {
    type Item = (usize, SoftAsciiChar);

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
            .map(|(idx, ch)| {
                (idx, SoftAsciiChar::from_unchecked(ch))
            })
    }

    #[inline]
    fn count(self) -> usize {
        self.inner.count()
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }

    fn last(self) -> Option<Self::Item> {
        self.inner.last()
            .map(|(idx, ch)| {
                (idx, SoftAsciiChar::from_unchecked(ch))
            })
    }
}

impl<'a> DoubleEndedIterator for SoftAsciiCharIndices<'a> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.inner.next_back()
            .map(|(idx, ch)| {
                (idx, SoftAsciiChar::from_unchecked(ch))
            })
    }
}

/// a wrapper around `Lines` turning each line into a `SoftAsciiStr`
///
/// This iterator is returned by `SoftAsciiChar::lines(&self)` instead
/// of `Lines`.
#[derive(Debug, Clone)]
pub struct SoftAsciiLines<'a> {
    inner: str::Lines<'a>
}

impl<'a> From<&'a SoftAsciiStr> for SoftAsciiLines<'a> {
    fn from(s: &'a SoftAsciiStr) -> SoftAsciiLines<'a> {
        SoftAsciiLines {
            inner: s.as_str().lines()
        }
    }
}

impl<'a> Iterator for SoftAsciiLines<'a> {
    type Item = &'a SoftAsciiStr;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
            .map(SoftAsciiStr::from_unchecked)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }

}

impl<'a> DoubleEndedIterator for SoftAsciiLines<'a> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.inner.next_back()
            .map(SoftAsciiStr::from_unchecked)
    }
}

/// a wrapper around `SplitWhitespace` turning each section into a `SoftAsciiStr`
///
/// This iterator is returned by `SoftAsciiChar::split_whitespace(&self)` instead
/// of `SplitWhitespace`.
#[derive(Clone)]
pub struct SoftAsciiSplitWhitespace<'a> {
    inner: str::SplitWhitespace<'a>
}


impl<'a> From<&'a SoftAsciiStr> for SoftAsciiSplitWhitespace<'a> {
    fn from(s: &'a SoftAsciiStr) -> SoftAsciiSplitWhitespace<'a> {
        SoftAsciiSplitWhitespace {
            inner: s.as_str().split_whitespace()
        }
    }
}

impl<'a> Iterator for SoftAsciiSplitWhitespace<'a> {
    type Item = &'a SoftAsciiStr;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
            .map(SoftAsciiStr::from_unchecked)
    }
}

impl<'a> DoubleEndedIterator for SoftAsciiSplitWhitespace<'a> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.inner.next_back()
            .map(SoftAsciiStr::from_unchecked)
    }
}


#[cfg(test)]
mod test {
    const UTF8_STR: &str = "❤ == <3";
    //TODO write tests for simple wrapper
    // (use some fuzzing like test library and make sure operation on
    //  `SoftAsciiStr` behave the same as on `str`)

    mod SoftAsciiStr {
        #![allow(non_snake_case)]
        use super::*;
        use super::super::SoftAsciiStr;
        use std::ops::{Range, RangeInclusive, RangeFrom, RangeTo, RangeToInclusive, RangeFull};

        #[test]
        fn from_str() {
            assert_eq!(
                SoftAsciiStr::from_str("hy ho\x00\x01\x02\x03").unwrap(),
                "hy ho\x00\x01\x02\x03"
            );
            assert!(SoftAsciiStr::from_str("↓").is_err());
        }

        #[test]
        fn from_unchecked() {
            assert_eq!(
                SoftAsciiStr::from_unchecked(UTF8_STR),
                UTF8_STR
            );
        }

        #[test]
        fn revalidate_soft_constraint() {
            let res = SoftAsciiStr::from_unchecked(UTF8_STR).revalidate_soft_constraint();
            assert_eq!(UTF8_STR, assert_err!(res).into_source());

            let res = SoftAsciiStr::from_unchecked("hy").revalidate_soft_constraint();
            let res: &SoftAsciiStr = assert_ok!(res);
            assert_eq!(
                res,
                "hy"
            );

        }

        #[test]
        fn compile_bounds__get_unchecked() {
            let _ = SoftAsciiStr::get_unchecked::<Range<usize>>;
            let _ = SoftAsciiStr::get_unchecked::<RangeInclusive<usize>>;
            let _ = SoftAsciiStr::get_unchecked::<RangeFrom<usize>>;
            let _ = SoftAsciiStr::get_unchecked::<RangeTo<usize>>;
            let _ = SoftAsciiStr::get_unchecked::<RangeToInclusive<usize>>;
            let _ = SoftAsciiStr::get_unchecked::<RangeFull>;
        }
    }

}