#[macro_use]
extern crate bob;

#[derive(Builder, Debug)]
#[builder_name = "Builder"]
#[builder_validate(validator = "Struct::validate", error = "BuildError")]
pub struct Struct {
    a: u32,
    b: i32,
}

#[derive(Debug)]
pub enum BuildError {
    CatastrophicFailure,
    CoreIntegrityException,
}

impl Struct {
    fn validate(self) -> Result<Self, BuildError> {
        Ok(self)
    }
}

#[derive(Builder, Debug)]
#[builder_name = "Builder2"]
#[builder_prefix = "set_"]
#[builder_validate(validator = "validate")]
struct Struct2<T: Eq> {
    pub a: T,
}

fn validate<T: Eq>(s: Struct2<T>) -> Struct2<T> {
    s
}

#[derive(Builder, Debug)]
#[builder_name = "Builder3"]
#[builder_rename(new = "create", build = "finish")]
pub struct Struct3<T: Eq> {
    pub a: Option<T>,
    b: Option<u32>,
    #[builder_prefix = "set_"]
    c: i32,
}

#[test]
fn build() {
    let built = Builder::new()
        .a(777)
        .b(-666)
        .build()
        .unwrap();
    assert_eq!(777, built.a);
    assert_eq!(-666, built.b);
    let built = Builder2::new()
        .set_a("Hello")
        .build();
    assert_eq!("Hello", built.a);
    let built = Builder3::create()
        .a("World")
        .set_c(-42)
        .finish();
    assert_eq!(Some("World"), built.a);
    assert_eq!(None, built.b);
    assert_eq!(-42, built.c);
}
