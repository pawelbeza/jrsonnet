//! Those implementations are a bit sketchy, as this is mostly performance experiments
//! of not yet finished nightly rust features

use std::{cell::RefCell, iter, mem::replace, rc::Rc};

use jrsonnet_gcmodule::{Cc, Trace};
use jrsonnet_interner::{IBytes, IStr};
use jrsonnet_parser::LocExpr;

use super::ArrValue;
use crate::{
	error::ErrorKind::InfiniteRecursionDetected,
	evaluate,
	function::FuncVal,
	val::{StrValue, ThunkValue},
	Context, Error, Result, Thunk, Val,
};

pub trait ArrayLike: Sized + Into<ArrValue> {
	#[cfg(feature = "nightly")]
	type Iter<'t>
	where
		Self: 't;
	#[cfg(feature = "nightly")]
	type IterLazy<'t>
	where
		Self: 't;
	#[cfg(feature = "nightly")]
	type IterCheap<'t>
	where
		Self: 't;

	fn len(&self) -> usize;
	fn is_empty(&self) -> bool {
		self.len() == 0
	}
	fn get(&self, index: usize) -> Result<Option<Val>>;
	fn get_lazy(&self, index: usize) -> Option<Thunk<Val>>;
	fn get_cheap(&self, index: usize) -> Option<Val>;
	#[cfg(feature = "nightly")]
	#[allow(clippy::iter_not_returning_iterator)]
	fn iter(&self) -> Self::Iter<'_>;
	#[cfg(feature = "nightly")]
	fn iter_lazy(&self) -> Self::IterLazy<'_>;
	#[cfg(feature = "nightly")]
	fn iter_cheap(&self) -> Option<Self::IterCheap<'_>>;

	fn reverse(self) -> ArrValue {
		ArrValue::Reverse(Cc::new(ReverseArray(self.into())))
	}
}

#[derive(Debug, Clone, Trace)]
pub struct SliceArray {
	pub(crate) inner: ArrValue,
	pub(crate) from: u32,
	pub(crate) to: u32,
	pub(crate) step: u32,
}

impl SliceArray {
	#[cfg(not(feature = "nightly"))]
	fn iter(&self) -> impl Iterator<Item = Result<Val>> + '_ {
		self.inner
			.iter()
			.skip(self.from as usize)
			.take((self.to - self.from) as usize)
			.step_by(self.step as usize)
	}

	#[cfg(not(feature = "nightly"))]
	fn iter_lazy(&self) -> impl Iterator<Item = Thunk<Val>> + '_ {
		self.inner
			.iter_lazy()
			.skip(self.from as usize)
			.take((self.to - self.from) as usize)
			.step_by(self.step as usize)
	}

	#[cfg(not(feature = "nightly"))]
	fn iter_cheap(&self) -> Option<impl crate::arr::ArrayLikeIter<Val> + '_> {
		Some(
			self.inner
				.iter_cheap()?
				.skip(self.from as usize)
				.take((self.to - self.from) as usize)
				.step_by(self.step as usize),
		)
	}
}
#[cfg(feature = "nightly")]
type SliceArrayIter<'t> = impl DoubleEndedIterator<Item = Result<Val>> + ExactSizeIterator + 't;
#[cfg(feature = "nightly")]
type SliceArrayLazyIter<'t> = impl DoubleEndedIterator<Item = Thunk<Val>> + ExactSizeIterator + 't;
#[cfg(feature = "nightly")]
type SliceArrayCheapIter<'t> = impl DoubleEndedIterator<Item = Val> + ExactSizeIterator + 't;
impl ArrayLike for SliceArray {
	#[cfg(feature = "nightly")]
	type Iter<'t> = SliceArrayIter<'t>;
	#[cfg(feature = "nightly")]
	type IterLazy<'t> = SliceArrayLazyIter<'t>;
	#[cfg(feature = "nightly")]
	type IterCheap<'t> = SliceArrayCheapIter<'t>;

	fn len(&self) -> usize {
		iter::repeat(())
			.take((self.to - self.from) as usize)
			.step_by(self.step as usize)
			.count()
	}

	fn get(&self, index: usize) -> Result<Option<Val>> {
		self.iter().nth(index).transpose()
	}

	fn get_lazy(&self, index: usize) -> Option<Thunk<Val>> {
		self.iter_lazy().nth(index)
	}

	fn get_cheap(&self, index: usize) -> Option<Val> {
		self.iter_cheap()?.nth(index)
	}

	#[cfg(feature = "nightly")]
	fn iter(&self) -> SliceArrayIter<'_> {
		self.inner
			.iter()
			.skip(self.from as usize)
			.take((self.to - self.from) as usize)
			.step_by(self.step as usize)
	}

	#[cfg(feature = "nightly")]
	fn iter_lazy(&self) -> SliceArrayLazyIter<'_> {
		self.inner
			.iter_lazy()
			.skip(self.from as usize)
			.take((self.to - self.from) as usize)
			.step_by(self.step as usize)
	}

	#[cfg(feature = "nightly")]
	fn iter_cheap(&self) -> Option<SliceArrayCheapIter<'_>> {
		Some(
			self.inner
				.iter_cheap()?
				.skip(self.from as usize)
				.take((self.to - self.from) as usize)
				.step_by(self.step as usize),
		)
	}
}
impl From<SliceArray> for ArrValue {
	fn from(value: SliceArray) -> Self {
		Self::Slice(Cc::new(value))
	}
}

#[derive(Trace, Debug, Clone)]
pub struct CharArray(pub Rc<Vec<char>>);
#[cfg(feature = "nightly")]
type CharArrayIter<'t> = impl DoubleEndedIterator<Item = Result<Val>> + ExactSizeIterator + 't;
#[cfg(feature = "nightly")]
type CharArrayLazyIter<'t> = impl DoubleEndedIterator<Item = Thunk<Val>> + ExactSizeIterator + 't;
#[cfg(feature = "nightly")]
type CharArrayCheapIter<'t> = impl DoubleEndedIterator<Item = Val> + ExactSizeIterator + 't;
impl ArrayLike for CharArray {
	#[cfg(feature = "nightly")]
	type Iter<'t> = CharArrayIter<'t>;
	#[cfg(feature = "nightly")]
	type IterLazy<'t> = CharArrayLazyIter<'t>;
	#[cfg(feature = "nightly")]
	type IterCheap<'t> = CharArrayCheapIter<'t>;

	fn len(&self) -> usize {
		self.0.len()
	}

	fn get(&self, index: usize) -> Result<Option<Val>> {
		Ok(self.get_cheap(index))
	}

	fn get_lazy(&self, index: usize) -> Option<Thunk<Val>> {
		self.get_cheap(index).map(Thunk::evaluated)
	}

	fn get_cheap(&self, index: usize) -> Option<Val> {
		self.0
			.get(index)
			.map(|v| Val::Str(StrValue::Flat(IStr::from(*v))))
	}

	#[cfg(feature = "nightly")]
	fn iter(&self) -> CharArrayIter<'_> {
		self.0
			.iter()
			.map(|v| Ok(Val::Str(StrValue::Flat(IStr::from(*v)))))
	}

	#[cfg(feature = "nightly")]
	fn iter_lazy(&self) -> CharArrayLazyIter<'_> {
		self.0
			.iter()
			.map(|v| Thunk::evaluated(Val::Str(StrValue::Flat(IStr::from(*v)))))
	}

	#[cfg(feature = "nightly")]
	fn iter_cheap(&self) -> Option<CharArrayCheapIter<'_>> {
		Some(
			self.0
				.iter()
				.map(|v| Val::Str(StrValue::Flat(IStr::from(*v)))),
		)
	}
}
impl From<CharArray> for ArrValue {
	fn from(value: CharArray) -> Self {
		ArrValue::Chars(value)
	}
}

#[derive(Trace, Debug, Clone)]
pub struct BytesArray(pub IBytes);
#[cfg(feature = "nightly")]
type BytesArrayIter<'t> = impl DoubleEndedIterator<Item = Result<Val>> + ExactSizeIterator + 't;
#[cfg(feature = "nightly")]
type BytesArrayLazyIter<'t> = impl DoubleEndedIterator<Item = Thunk<Val>> + ExactSizeIterator + 't;
#[cfg(feature = "nightly")]
type BytesArrayCheapIter<'t> = impl DoubleEndedIterator<Item = Val> + ExactSizeIterator + 't;
impl ArrayLike for BytesArray {
	#[cfg(feature = "nightly")]
	type Iter<'t> = BytesArrayIter<'t>;
	#[cfg(feature = "nightly")]
	type IterLazy<'t> = BytesArrayLazyIter<'t>;
	#[cfg(feature = "nightly")]
	type IterCheap<'t> = BytesArrayCheapIter<'t>;

	fn len(&self) -> usize {
		self.0.len()
	}

	fn get(&self, index: usize) -> Result<Option<Val>> {
		Ok(self.get_cheap(index))
	}

	fn get_lazy(&self, index: usize) -> Option<Thunk<Val>> {
		self.get_cheap(index).map(Thunk::evaluated)
	}

	fn get_cheap(&self, index: usize) -> Option<Val> {
		self.0.get(index).map(|v| Val::Num(f64::from(*v)))
	}

	#[cfg(feature = "nightly")]
	fn iter(&self) -> BytesArrayIter<'_> {
		self.0.iter().map(|v| Ok(Val::Num(f64::from(*v))))
	}

	#[cfg(feature = "nightly")]
	fn iter_lazy(&self) -> BytesArrayLazyIter<'_> {
		self.0
			.iter()
			.map(|v| Thunk::evaluated(Val::Num(f64::from(*v))))
	}

	#[cfg(feature = "nightly")]
	fn iter_cheap(&self) -> Option<BytesArrayCheapIter<'_>> {
		Some(self.0.iter().map(|v| Val::Num(f64::from(*v))))
	}
}
impl From<BytesArray> for ArrValue {
	fn from(value: BytesArray) -> Self {
		ArrValue::Bytes(value)
	}
}

#[derive(Debug, Trace, Clone)]
enum ArrayThunk<T: 'static + Trace> {
	Computed(Val),
	Errored(Error),
	Waiting(T),
	Pending,
}

#[derive(Debug, Trace)]
pub struct ExprArrayInner {
	ctx: Context,
	cached: RefCell<Vec<ArrayThunk<LocExpr>>>,
}
#[derive(Debug, Trace, Clone)]
pub struct ExprArray(pub Cc<ExprArrayInner>);
impl ExprArray {
	pub fn new(ctx: Context, items: impl IntoIterator<Item = LocExpr>) -> Self {
		Self(Cc::new(ExprArrayInner {
			ctx,
			cached: RefCell::new(items.into_iter().map(ArrayThunk::Waiting).collect()),
		}))
	}
}
#[cfg(feature = "nightly")]
type ExprArrayIter<'t> = impl DoubleEndedIterator<Item = Result<Val>> + ExactSizeIterator + 't;
#[cfg(feature = "nightly")]
type ExprArrayLazyIter<'t> = impl DoubleEndedIterator<Item = Thunk<Val>> + ExactSizeIterator + 't;
#[cfg(feature = "nightly")]
type ExprArrayCheapIter<'t> = iter::Empty<Val>;
impl ArrayLike for ExprArray {
	#[cfg(feature = "nightly")]
	type Iter<'t> = ExprArrayIter<'t>;
	#[cfg(feature = "nightly")]
	type IterLazy<'t> = ExprArrayLazyIter<'t>;
	#[cfg(feature = "nightly")]
	type IterCheap<'t> = ExprArrayCheapIter<'t>;

	fn len(&self) -> usize {
		self.0.cached.borrow().len()
	}
	fn get(&self, index: usize) -> Result<Option<Val>> {
		if index >= self.len() {
			return Ok(None);
		}
		match &self.0.cached.borrow()[index] {
			ArrayThunk::Computed(c) => return Ok(Some(c.clone())),
			ArrayThunk::Errored(e) => return Err(e.clone()),
			ArrayThunk::Pending => return Err(InfiniteRecursionDetected.into()),
			ArrayThunk::Waiting(..) => {}
		};

		let ArrayThunk::Waiting(expr) = replace(&mut self.0.cached.borrow_mut()[index], ArrayThunk::Pending) else {
			unreachable!()
		};

		let new_value = match evaluate(self.0.ctx.clone(), &expr) {
			Ok(v) => v,
			Err(e) => {
				self.0.cached.borrow_mut()[index] = ArrayThunk::Errored(e.clone());
				return Err(e);
			}
		};
		self.0.cached.borrow_mut()[index] = ArrayThunk::Computed(new_value.clone());
		Ok(Some(new_value))
	}
	fn get_lazy(&self, index: usize) -> Option<Thunk<Val>> {
		#[derive(Trace)]
		struct ArrayElement {
			arr_thunk: ExprArray,
			index: usize,
		}

		impl ThunkValue for ArrayElement {
			type Output = Val;

			fn get(self: Box<Self>) -> Result<Self::Output> {
				self.arr_thunk
					.get(self.index)
					.transpose()
					.expect("index checked")
			}
		}

		if index >= self.len() {
			return None;
		}
		match &self.0.cached.borrow()[index] {
			ArrayThunk::Computed(c) => return Some(Thunk::evaluated(c.clone())),
			ArrayThunk::Errored(e) => return Some(Thunk::errored(e.clone())),
			ArrayThunk::Waiting(_) | ArrayThunk::Pending => {}
		};

		Some(Thunk::new(ArrayElement {
			arr_thunk: self.clone(),
			index,
		}))
	}
	fn get_cheap(&self, _index: usize) -> Option<Val> {
		None
	}

	#[cfg(feature = "nightly")]
	fn iter(&self) -> ExprArrayIter<'_> {
		(0..self.len()).map(|i| self.get(i).transpose().expect("index checked"))
	}
	#[cfg(feature = "nightly")]
	fn iter_lazy(&self) -> ExprArrayLazyIter<'_> {
		(0..self.len()).map(|i| self.get_lazy(i).expect("index checked"))
	}
	#[cfg(feature = "nightly")]
	fn iter_cheap(&self) -> Option<Self::IterCheap<'_>> {
		None
	}
}
impl From<ExprArray> for ArrValue {
	fn from(value: ExprArray) -> Self {
		Self::Expr(value)
	}
}

#[derive(Trace, Debug, Clone)]
pub struct ExtendedArray {
	pub a: ArrValue,
	pub b: ArrValue,
	split: usize,
	len: usize,
}
#[cfg(feature = "nightly")]

type ExtendedArrayIter<'t> = impl DoubleEndedIterator<Item = Result<Val>> + ExactSizeIterator + 't;
#[cfg(feature = "nightly")]
type ExtendedArrayLazyIter<'t> =
	impl DoubleEndedIterator<Item = Thunk<Val>> + ExactSizeIterator + 't;
#[cfg(feature = "nightly")]
type ExtendedArrayCheapIter<'t> = impl DoubleEndedIterator<Item = Val> + ExactSizeIterator + 't;
impl ExtendedArray {
	pub fn new(a: ArrValue, b: ArrValue) -> Self {
		let a_len = a.len();
		let b_len = b.len();
		Self {
			a,
			b,
			split: a_len,
			len: a_len.checked_add(b_len).expect("too large array value"),
		}
	}
}

struct WithExactSize<I>(I, usize);
impl<I, T> Iterator for WithExactSize<I>
where
	I: Iterator<Item = T>,
{
	type Item = T;

	fn next(&mut self) -> Option<Self::Item> {
		self.0.next()
	}
	fn nth(&mut self, n: usize) -> Option<Self::Item> {
		self.0.nth(n)
	}
	fn size_hint(&self) -> (usize, Option<usize>) {
		(self.1, Some(self.1))
	}
}
impl<I> DoubleEndedIterator for WithExactSize<I>
where
	I: DoubleEndedIterator,
{
	fn next_back(&mut self) -> Option<Self::Item> {
		self.0.next_back()
	}
	fn nth_back(&mut self, n: usize) -> Option<Self::Item> {
		self.0.nth_back(n)
	}
}
impl<I> ExactSizeIterator for WithExactSize<I>
where
	I: Iterator,
{
	fn len(&self) -> usize {
		self.1
	}
}
impl ArrayLike for ExtendedArray {
	#[cfg(feature = "nightly")]
	type Iter<'t> = ExtendedArrayIter<'t>;
	#[cfg(feature = "nightly")]
	type IterLazy<'t> = ExtendedArrayLazyIter<'t>;
	#[cfg(feature = "nightly")]
	type IterCheap<'t> = ExtendedArrayCheapIter<'t>;

	fn get(&self, index: usize) -> Result<Option<Val>> {
		if self.split > index {
			self.a.get(index)
		} else {
			self.b.get(index - self.split)
		}
	}
	fn get_lazy(&self, index: usize) -> Option<Thunk<Val>> {
		if self.split > index {
			self.a.get_lazy(index)
		} else {
			self.b.get_lazy(index - self.split)
		}
	}

	fn len(&self) -> usize {
		self.len
	}

	fn get_cheap(&self, index: usize) -> Option<Val> {
		if self.split > index {
			self.a.get_cheap(index)
		} else {
			self.b.get_cheap(index - self.split)
		}
	}

	#[cfg(feature = "nightly")]
	fn iter(&self) -> ExtendedArrayIter<'_> {
		WithExactSize(self.a.iter().chain(self.b.iter()), self.len)
	}
	#[cfg(feature = "nightly")]
	fn iter_lazy(&self) -> ExtendedArrayLazyIter<'_> {
		WithExactSize(self.a.iter_lazy().chain(self.b.iter_lazy()), self.len)
	}
	#[cfg(feature = "nightly")]
	fn iter_cheap(&self) -> Option<ExtendedArrayCheapIter<'_>> {
		let a = self.a.iter_cheap()?;
		let b = self.b.iter_cheap()?;
		Some(WithExactSize(a.chain(b), self.len))
	}
}
impl From<ExtendedArray> for ArrValue {
	fn from(value: ExtendedArray) -> Self {
		Self::Extended(Cc::new(value))
	}
}

#[derive(Trace, Debug, Clone)]
pub struct LazyArray(pub Cc<Vec<Thunk<Val>>>);
#[cfg(feature = "nightly")]
type LazyArrayIter<'t> = impl DoubleEndedIterator<Item = Result<Val>> + ExactSizeIterator + 't;
#[cfg(feature = "nightly")]
type LazyArrayLazyIter<'t> = impl DoubleEndedIterator<Item = Thunk<Val>> + ExactSizeIterator + 't;
#[cfg(feature = "nightly")]
type LazyArrayCheapIter<'t> = iter::Empty<Val>;
impl ArrayLike for LazyArray {
	#[cfg(feature = "nightly")]
	type Iter<'t> = LazyArrayIter<'t>;

	#[cfg(feature = "nightly")]
	type IterLazy<'t> = LazyArrayLazyIter<'t>;

	#[cfg(feature = "nightly")]
	type IterCheap<'t> = LazyArrayCheapIter<'t>;

	fn len(&self) -> usize {
		self.0.len()
	}
	fn get(&self, index: usize) -> Result<Option<Val>> {
		let Some(v) = self.0.get(index) else {
			return Ok(None);
		};
		v.evaluate().map(Some)
	}
	fn get_cheap(&self, _index: usize) -> Option<Val> {
		None
	}
	fn get_lazy(&self, index: usize) -> Option<Thunk<Val>> {
		self.0.get(index).cloned()
	}
	#[cfg(feature = "nightly")]
	fn iter(&self) -> LazyArrayIter<'_> {
		self.0.iter().map(Thunk::evaluate)
	}
	#[cfg(feature = "nightly")]
	fn iter_lazy(&self) -> LazyArrayLazyIter<'_> {
		self.0.iter().cloned()
	}
	#[cfg(feature = "nightly")]
	fn iter_cheap(&self) -> Option<LazyArrayCheapIter<'_>> {
		None
	}
}
impl From<LazyArray> for ArrValue {
	fn from(value: LazyArray) -> Self {
		Self::Lazy(value)
	}
}

#[derive(Trace, Debug, Clone)]
pub struct EagerArray(pub Cc<Vec<Val>>);
#[cfg(feature = "nightly")]
type EagerArrayIter<'t> = impl DoubleEndedIterator<Item = Result<Val>> + ExactSizeIterator + 't;
#[cfg(feature = "nightly")]
type EagerArrayLazyIter<'t> = impl DoubleEndedIterator<Item = Thunk<Val>> + ExactSizeIterator + 't;
#[cfg(feature = "nightly")]
type EagerArrayCheapIter<'t> = impl DoubleEndedIterator<Item = Val> + ExactSizeIterator + 't;
impl ArrayLike for EagerArray {
	#[cfg(feature = "nightly")]
	type Iter<'t> = EagerArrayIter<'t>;

	#[cfg(feature = "nightly")]
	type IterLazy<'t> = EagerArrayLazyIter<'t>;

	#[cfg(feature = "nightly")]
	type IterCheap<'t> = EagerArrayCheapIter<'t>;

	fn len(&self) -> usize {
		self.0.len()
	}

	fn get(&self, index: usize) -> Result<Option<Val>> {
		Ok(self.0.get(index).cloned())
	}

	fn get_lazy(&self, index: usize) -> Option<Thunk<Val>> {
		self.0.get(index).cloned().map(Thunk::evaluated)
	}

	fn get_cheap(&self, index: usize) -> Option<Val> {
		self.0.get(index).cloned()
	}

	#[cfg(feature = "nightly")]
	fn iter(&self) -> EagerArrayIter<'_> {
		self.0.iter().cloned().map(Ok)
	}

	#[cfg(feature = "nightly")]
	fn iter_lazy(&self) -> EagerArrayLazyIter<'_> {
		self.0.iter().cloned().map(Thunk::evaluated)
	}

	#[cfg(feature = "nightly")]
	fn iter_cheap(&self) -> Option<EagerArrayCheapIter<'_>> {
		Some(self.0.iter().cloned())
	}
}
impl From<EagerArray> for ArrValue {
	fn from(value: EagerArray) -> Self {
		Self::Eager(value)
	}
}

/// Inclusive range type
#[derive(Debug, Trace, Clone, PartialEq, Eq)]
pub struct RangeArray {
	start: i32,
	end: i32,
}
impl RangeArray {
	pub fn empty() -> Self {
		Self::new_exclusive(0, 0)
	}
	pub fn new_exclusive(start: i32, end: i32) -> Self {
		end.checked_sub(1)
			.map_or_else(Self::empty, |end| Self { start, end })
	}
	pub fn new_inclusive(start: i32, end: i32) -> Self {
		Self { start, end }
	}
	fn range(&self) -> impl Iterator<Item = i32> + ExactSizeIterator + DoubleEndedIterator {
		WithExactSize(
			self.start..=self.end,
			(self.end as usize)
				.wrapping_sub(self.start as usize)
				.wrapping_add(1),
		)
	}
}

#[cfg(feature = "nightly")]
type RangeArrayIter<'t> = impl DoubleEndedIterator<Item = Result<Val>> + ExactSizeIterator + 't;
#[cfg(feature = "nightly")]
type RangeArrayLazyIter<'t> = impl DoubleEndedIterator<Item = Thunk<Val>> + ExactSizeIterator + 't;
#[cfg(feature = "nightly")]
type RangeArrayCheapIter<'t> = impl DoubleEndedIterator<Item = Val> + ExactSizeIterator + 't;
impl ArrayLike for RangeArray {
	#[cfg(feature = "nightly")]
	type Iter<'t> = RangeArrayIter<'t>;

	#[cfg(feature = "nightly")]
	type IterLazy<'t> = RangeArrayLazyIter<'t>;

	#[cfg(feature = "nightly")]
	type IterCheap<'t> = RangeArrayCheapIter<'t>;

	fn len(&self) -> usize {
		self.range().len()
	}
	fn is_empty(&self) -> bool {
		self.range().len() == 0
	}

	fn get(&self, index: usize) -> Result<Option<Val>> {
		Ok(self.get_cheap(index))
	}

	fn get_lazy(&self, index: usize) -> Option<Thunk<Val>> {
		self.get_cheap(index).map(Thunk::evaluated)
	}

	fn get_cheap(&self, index: usize) -> Option<Val> {
		self.range().nth(index).map(|i| Val::Num(f64::from(i)))
	}

	#[cfg(feature = "nightly")]
	fn iter(&self) -> RangeArrayIter<'_> {
		self.range().map(|i| Ok(Val::Num(f64::from(i))))
	}

	#[cfg(feature = "nightly")]
	fn iter_lazy(&self) -> RangeArrayLazyIter<'_> {
		self.range()
			.map(|i| Thunk::evaluated(Val::Num(f64::from(i))))
	}

	#[cfg(feature = "nightly")]
	fn iter_cheap(&self) -> Option<RangeArrayCheapIter<'_>> {
		Some(self.range().map(|i| Val::Num(f64::from(i))))
	}
}
impl From<RangeArray> for ArrValue {
	fn from(value: RangeArray) -> Self {
		Self::Range(value)
	}
}

#[derive(Debug, Trace, Clone)]
pub struct ReverseArray(pub ArrValue);
impl ArrayLike for ReverseArray {
	#[cfg(feature = "nightly")]
	type Iter<'t> = iter::Rev<UnknownArrayIter<'t>>;

	#[cfg(feature = "nightly")]
	type IterLazy<'t> = iter::Rev<UnknownArrayIterLazy<'t>>;

	#[cfg(feature = "nightly")]
	type IterCheap<'t> = iter::Rev<UnknownArrayIterCheap<'t>>;

	fn len(&self) -> usize {
		self.0.len()
	}

	fn get(&self, index: usize) -> Result<Option<Val>> {
		self.0.get(self.0.len() - index - 1)
	}

	fn get_lazy(&self, index: usize) -> Option<Thunk<Val>> {
		self.0.get_lazy(self.0.len() - index - 1)
	}

	fn get_cheap(&self, index: usize) -> Option<Val> {
		self.0.get_cheap(self.0.len() - index - 1)
	}

	#[cfg(feature = "nightly")]
	fn iter(&self) -> iter::Rev<UnknownArrayIter<'_>> {
		self.0.iter().rev()
	}

	#[cfg(feature = "nightly")]
	fn iter_lazy(&self) -> iter::Rev<UnknownArrayIterLazy<'_>> {
		self.0.iter_lazy().rev()
	}

	#[cfg(feature = "nightly")]
	fn iter_cheap(&self) -> Option<iter::Rev<UnknownArrayIterCheap<'_>>> {
		Some(self.0.iter_cheap()?.rev())
	}
	fn reverse(self) -> ArrValue {
		self.0
	}
}
impl From<ReverseArray> for ArrValue {
	fn from(value: ReverseArray) -> Self {
		Self::Reverse(Cc::new(value))
	}
}

#[derive(Trace, Debug)]
pub struct MappedArrayInner {
	inner: ArrValue,
	cached: RefCell<Vec<ArrayThunk<()>>>,
	mapper: FuncVal,
}
#[derive(Trace, Debug, Clone)]
pub struct MappedArray(Cc<MappedArrayInner>);
impl MappedArray {
	pub fn new(inner: ArrValue, mapper: FuncVal) -> Self {
		let len = inner.len();
		Self(Cc::new(MappedArrayInner {
			inner,
			cached: RefCell::new(vec![ArrayThunk::Waiting(()); len]),
			mapper,
		}))
	}
}
#[cfg(feature = "nightly")]
type MappedArrayIter<'t> = impl DoubleEndedIterator<Item = Result<Val>> + ExactSizeIterator + 't;
#[cfg(feature = "nightly")]
type MappedArrayLazyIter<'t> = impl DoubleEndedIterator<Item = Thunk<Val>> + ExactSizeIterator + 't;
#[cfg(feature = "nightly")]
type MappedArrayCheapIter<'t> = iter::Empty<Val>;
impl ArrayLike for MappedArray {
	#[cfg(feature = "nightly")]
	type Iter<'t> = MappedArrayIter<'t>;
	#[cfg(feature = "nightly")]
	type IterLazy<'t> = MappedArrayLazyIter<'t>;
	#[cfg(feature = "nightly")]
	type IterCheap<'t> = MappedArrayCheapIter<'t>;

	fn len(&self) -> usize {
		self.0.cached.borrow().len()
	}

	fn get(&self, index: usize) -> Result<Option<Val>> {
		if index >= self.len() {
			return Ok(None);
		}
		match &self.0.cached.borrow()[index] {
			ArrayThunk::Computed(c) => return Ok(Some(c.clone())),
			ArrayThunk::Errored(e) => return Err(e.clone()),
			ArrayThunk::Pending => return Err(InfiniteRecursionDetected.into()),
			ArrayThunk::Waiting(..) => {}
		};

		let ArrayThunk::Waiting(_) = replace(&mut self.0.cached.borrow_mut()[index], ArrayThunk::Pending) else {
			unreachable!()
		};

		let val = self
			.0
			.inner
			.get(index)
			.transpose()
			.expect("index checked")
			.and_then(|r| self.0.mapper.evaluate_simple(&(r,), false));

		let new_value = match val {
			Ok(v) => v,
			Err(e) => {
				self.0.cached.borrow_mut()[index] = ArrayThunk::Errored(e.clone());
				return Err(e);
			}
		};
		self.0.cached.borrow_mut()[index] = ArrayThunk::Computed(new_value.clone());
		Ok(Some(new_value))
	}
	fn get_lazy(&self, index: usize) -> Option<Thunk<Val>> {
		#[derive(Trace)]
		struct ArrayElement {
			arr_thunk: MappedArray,
			index: usize,
		}

		impl ThunkValue for ArrayElement {
			type Output = Val;

			fn get(self: Box<Self>) -> Result<Self::Output> {
				self.arr_thunk
					.get(self.index)
					.transpose()
					.expect("index checked")
			}
		}

		if index >= self.len() {
			return None;
		}
		match &self.0.cached.borrow()[index] {
			ArrayThunk::Computed(c) => return Some(Thunk::evaluated(c.clone())),
			ArrayThunk::Errored(e) => return Some(Thunk::errored(e.clone())),
			ArrayThunk::Waiting(_) | ArrayThunk::Pending => {}
		};

		Some(Thunk::new(ArrayElement {
			arr_thunk: self.clone(),
			index,
		}))
	}

	fn get_cheap(&self, _index: usize) -> Option<Val> {
		None
	}

	#[cfg(feature = "nightly")]
	fn iter(&self) -> MappedArrayIter<'_> {
		(0..self.len()).map(|i| self.get(i).transpose().expect("length checked"))
	}

	#[cfg(feature = "nightly")]
	fn iter_lazy(&self) -> MappedArrayLazyIter<'_> {
		(0..self.len()).map(|i| self.get_lazy(i).expect("length checked"))
	}

	#[cfg(feature = "nightly")]
	fn iter_cheap(&self) -> Option<Self::IterCheap<'_>> {
		None
	}
}
impl From<MappedArray> for ArrValue {
	fn from(value: MappedArray) -> Self {
		Self::Mapped(value)
	}
}

#[derive(Trace, Debug)]
pub struct RepeatedArrayInner {
	data: ArrValue,
	repeats: usize,
	total_len: usize,
}
#[derive(Trace, Debug, Clone)]
pub struct RepeatedArray(Cc<RepeatedArrayInner>);
impl RepeatedArray {
	pub fn new(data: ArrValue, repeats: usize) -> Option<Self> {
		let total_len = data.len().checked_mul(repeats)?;
		Some(Self(Cc::new(RepeatedArrayInner {
			data,
			repeats,
			total_len,
		})))
	}
	pub fn is_cheap(&self) -> bool {
		self.0.data.is_cheap()
	}
}

#[cfg(feature = "nightly")]
type RepeatedArrayIter<'t> = impl DoubleEndedIterator<Item = Result<Val>> + ExactSizeIterator + 't;
#[cfg(feature = "nightly")]
type RepeatedArrayLazyIter<'t> =
	impl DoubleEndedIterator<Item = Thunk<Val>> + ExactSizeIterator + 't;
#[cfg(feature = "nightly")]
type RepeatedArrayCheapIter<'t> = impl DoubleEndedIterator<Item = Val> + ExactSizeIterator + 't;
impl ArrayLike for RepeatedArray {
	#[cfg(feature = "nightly")]
	type Iter<'t> = RepeatedArrayIter<'t>;
	#[cfg(feature = "nightly")]
	type IterLazy<'t> = RepeatedArrayLazyIter<'t>;
	#[cfg(feature = "nightly")]
	type IterCheap<'t> = RepeatedArrayCheapIter<'t>;

	fn len(&self) -> usize {
		self.0.total_len
	}

	fn get(&self, index: usize) -> Result<Option<Val>> {
		if index > self.0.total_len {
			return Ok(None);
		}
		self.0.data.get(index % self.0.data.len())
	}

	fn get_lazy(&self, index: usize) -> Option<Thunk<Val>> {
		if index > self.0.total_len {
			return None;
		}
		self.0.data.get_lazy(index % self.0.data.len())
	}

	fn get_cheap(&self, index: usize) -> Option<Val> {
		if index > self.0.total_len {
			return None;
		}
		self.0.data.get_cheap(index % self.0.data.len())
	}

	#[cfg(feature = "nightly")]
	fn iter(&self) -> RepeatedArrayIter<'_> {
		(0..self.0.total_len)
			.map(|i| self.get(i))
			.map(Result::transpose)
			.map(Option::unwrap)
	}

	#[cfg(feature = "nightly")]
	fn iter_lazy(&self) -> RepeatedArrayLazyIter<'_> {
		(0..self.0.total_len)
			.map(|i| self.get_lazy(i))
			.map(Option::unwrap)
	}

	#[cfg(feature = "nightly")]
	fn iter_cheap(&self) -> Option<RepeatedArrayCheapIter<'_>> {
		if !self.0.data.is_cheap() {
			return None;
		}
		Some(
			(0..self.0.total_len)
				.map(|i| self.get_cheap(i))
				.map(Option::unwrap),
		)
	}
}
impl From<RepeatedArray> for ArrValue {
	fn from(value: RepeatedArray) -> Self {
		Self::Repeated(value)
	}
}

#[cfg(feature = "nightly")]
macro_rules! impl_iter_enum {
	($n:ident => $v:ident) => {
		pub enum $n<'t> {
			Bytes(<BytesArray as ArrayLike>::$v<'t>),
			Expr(<ExprArray as ArrayLike>::$v<'t>),
			Lazy(<LazyArray as ArrayLike>::$v<'t>),
			Eager(<EagerArray as ArrayLike>::$v<'t>),
			Range(<RangeArray as ArrayLike>::$v<'t>),
			Slice(Box<<SliceArray as ArrayLike>::$v<'t>>),
			Extended(Box<<ExtendedArray as ArrayLike>::$v<'t>>),
			Reverse(Box<<ReverseArray as ArrayLike>::$v<'t>>),
			Mapped(Box<<MappedArray as ArrayLike>::$v<'t>>),
			Repeated(Box<<RepeatedArray as ArrayLike>::$v<'t>>),
		}
	};
}

macro_rules! pass {
	($t:ident.$m:ident($($ident:ident),*)) => {
		match $t {
			Self::Bytes(e) => e.$m($($ident)*),
			Self::Chars(e) => e.$m($($ident)*),
			Self::Expr(e) => e.$m($($ident)*),
			Self::Lazy(e) => e.$m($($ident)*),
			Self::Eager(e) => e.$m($($ident)*),
			Self::Range(e) => e.$m($($ident)*),
			Self::Slice(e) => e.$m($($ident)*),
			Self::Extended(e) => e.$m($($ident)*),
			Self::Reverse(e) => e.$m($($ident)*),
			Self::Mapped(e) => e.$m($($ident)*),
			Self::Repeated(e) => e.$m($($ident)*),
		}
	};
}
pub(super) use pass;

#[cfg(feature = "nightly")]
macro_rules! pass_iter_call {
	($t:ident.$c:ident $(in $wrap:ident)? => $e:ident) => {
		match $t {
			ArrValue::Bytes(e) => $e::Bytes($($wrap!)?(e.$c())),
			ArrValue::Lazy(e) => $e::Lazy($($wrap!)?(e.$c())),
			ArrValue::Expr(e) => $e::Expr($($wrap!)?(e.$c())),
			ArrValue::Eager(e) => $e::Eager($($wrap!)?(e.$c())),
			ArrValue::Range(e) => $e::Range($($wrap!)?(e.$c())),
			ArrValue::Slice(e) => $e::Slice(Box::new($($wrap!)?(e.$c()))),
			ArrValue::Extended(e) => $e::Extended(Box::new($($wrap!)?(e.$c()))),
			ArrValue::Reverse(e) => $e::Reverse(Box::new($($wrap!)?(e.$c()))),
			ArrValue::Mapped(e) => $e::Mapped(Box::new($($wrap!)?(e.$c()))),
			ArrValue::Repeated(e) => $e::Repeated(Box::new($($wrap!)?(e.$c()))),
		}
	};
}
#[cfg(feature = "nightly")]
pub(super) use pass_iter_call;

#[cfg(feature = "nightly")]
macro_rules! impl_iter {
	($t:ident => $out:ty) => {
		impl Iterator for $t<'_> {
			type Item = $out;

			fn next(&mut self) -> Option<Self::Item> {
				pass!(self.next())
			}
			fn nth(&mut self, count: usize) -> Option<Self::Item> {
				pass!(self.nth(count))
			}
			fn size_hint(&self) -> (usize, Option<usize>) {
				pass!(self.size_hint())
			}
		}
		impl DoubleEndedIterator for $t<'_> {
			fn next_back(&mut self) -> Option<Self::Item> {
				pass!(self.next_back())
			}
			fn nth_back(&mut self, count: usize) -> Option<Self::Item> {
				pass!(self.nth_back(count))
			}
		}
		impl ExactSizeIterator for $t<'_> {
			fn len(&self) -> usize {
				match self {
					Self::Bytes(e) => e.len(),
					Self::Expr(e) => e.len(),
					Self::Lazy(e) => e.len(),
					Self::Eager(e) => e.len(),
					Self::Range(e) => e.len(),
					Self::Slice(e) => e.len(),
					Self::Extended(e) => {
						e.size_hint().1.expect("overflow is checked in constructor")
					}
					Self::Reverse(e) => e.len(),
					Self::Mapped(e) => e.len(),
					Self::Repeated(e) => e.len(),
				}
			}
		}
	};
}

#[cfg(feature = "nightly")]
impl_iter_enum!(UnknownArrayIter => Iter);
#[cfg(feature = "nightly")]
impl_iter!(UnknownArrayIter => Result<Val>);
#[cfg(feature = "nightly")]
impl_iter_enum!(UnknownArrayIterLazy => IterLazy);
#[cfg(feature = "nightly")]
impl_iter!(UnknownArrayIterLazy => Thunk<Val>);
#[cfg(feature = "nightly")]
impl_iter_enum!(UnknownArrayIterCheap => IterCheap);
#[cfg(feature = "nightly")]
impl_iter!(UnknownArrayIterCheap => Val);
