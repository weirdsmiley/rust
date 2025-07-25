//@ only-x86_64

#![warn(unused_attributes)]

#[target_feature(enable = "sse2")]
//~^ ERROR attribute should be applied to a function
extern crate alloc;
//~^ NOTE not a function

#[target_feature(enable = "sse2")]
//~^ ERROR attribute should be applied to a function
use alloc::alloc::alloc;
//~^ NOTE not a function

#[target_feature(enable = "sse2")]
//~^ ERROR attribute should be applied to a function
extern "Rust" {}
//~^ NOTE not a function

#[target_feature = "+sse2"]
//~^ ERROR malformed `target_feature` attribute
//~| NOTE expected this to be a list
#[target_feature(enable = "foo")]
//~^ ERROR not valid for this target
//~| NOTE `foo` is not valid for this target
#[target_feature(bar)]
//~^ ERROR malformed `target_feature` attribute
//~| NOTE expected this to be of the form `enable = "..."`
#[target_feature(disable = "baz")]
//~^ ERROR malformed `target_feature` attribute
//~| NOTE expected this to be of the form `enable = "..."`
unsafe fn foo() {}

#[target_feature(enable = "sse2")]
//~^ ERROR attribute should be applied to a function
mod another {}
//~^ NOTE not a function

#[target_feature(enable = "sse2")]
//~^ ERROR attribute should be applied to a function
const FOO: usize = 7;
//~^ NOTE not a function

#[target_feature(enable = "sse2")]
//~^ ERROR attribute should be applied to a function
struct Foo;
//~^ NOTE not a function

#[target_feature(enable = "sse2")]
//~^ ERROR attribute should be applied to a function
enum Bar {}
//~^ NOTE not a function

#[target_feature(enable = "sse2")]
//~^ ERROR attribute should be applied to a function
union Qux {
    //~^ NOTE not a function
    f1: u16,
    f2: u16,
}

#[target_feature(enable = "sse2")]
//~^ ERROR attribute should be applied to a function
type Uwu = ();
//~^ NOTE not a function

#[target_feature(enable = "sse2")]
//~^ ERROR attribute should be applied to a function
trait Baz {}
//~^ NOTE not a function

#[inline(always)]
//~^ ERROR: cannot use `#[inline(always)]`
#[target_feature(enable = "sse2")]
unsafe fn test() {}

#[target_feature(enable = "sse2")]
//~^ ERROR attribute should be applied to a function
static A: () = ();
//~^ NOTE not a function

#[target_feature(enable = "sse2")]
//~^ ERROR attribute should be applied to a function
impl Quux for u8 {}
//~^ NOTE not a function
//~| NOTE missing `foo` in implementation
//~| ERROR missing: `foo`

#[target_feature(enable = "sse2")]
//~^ ERROR attribute should be applied to a function
impl Foo {}
//~^ NOTE not a function

trait Quux {
    fn foo(); //~ NOTE `foo` from trait
    //~^ NOTE: type in trait
}

impl Quux for Foo {
    #[target_feature(enable = "sse2")]
    //~^ ERROR `#[target_feature(..)]` cannot be applied to safe trait method
    //~| NOTE cannot be applied to safe trait method
    fn foo() {}
    //~^ NOTE not an `unsafe` function
    //~| ERROR: incompatible type for trait
    //~| NOTE: expected safe fn, found unsafe fn
    //~| NOTE: expected signature `fn()`
}

fn main() {
    #[target_feature(enable = "sse2")]
    //~^ ERROR attribute should be applied to a function
    unsafe {
        foo();
    }
    //~^^^ NOTE not a function

    #[target_feature(enable = "sse2")]
    //~^ ERROR attribute should be applied to a function
    || {};
    //~^ NOTE not a function
}

#[target_feature(enable = "+sse2")]
//~^ ERROR `+sse2` is not valid for this target
//~| NOTE `+sse2` is not valid for this target
unsafe fn hey() {}
