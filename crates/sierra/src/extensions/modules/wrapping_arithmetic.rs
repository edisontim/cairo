use std::marker::PhantomData;

use super::non_zero::NonZeroType;
use crate::define_concrete_libfunc_hierarchy;
use crate::extensions::lib_func::{
    DeferredOutputKind, LibFuncSignature, OutputVarInfo, SierraApChange,
    SignatureSpecializationContext, SpecializationContext,
};
use crate::extensions::{
    GenericLibFunc, NamedLibFunc, NamedType, OutputVarReferenceInfo, SignatureBasedConcreteLibFunc,
    SpecializationError,
};
use crate::ids::{GenericLibFuncId, GenericTypeId};
use crate::program::GenericArg;

/// Possible arithmetic operators.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Operator {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
}

/// Trait for implementing wrapping arithmetic operations for an arithmetic type.
pub trait WrappingArithmeticTraits: Default {
    /// The add library function id.
    const ADD: GenericLibFuncId;
    /// The sub library function id.
    const SUB: GenericLibFuncId;
    /// The mul library function id.
    const MUL: GenericLibFuncId;
    /// The div library function id.
    const DIV: GenericLibFuncId;
    /// The mod library function id.
    const MOD: GenericLibFuncId;
    /// The const library function id.
    const CONST: GenericLibFuncId;
    /// The id of the generic type to implement the library functions for.
    const GENERIC_TYPE_ID: GenericTypeId;
}

/// Libfunc for arithmetic operations.
pub struct OperationLibFunc<TArithmeticTraits: WrappingArithmeticTraits> {
    pub operator: Operator,
    _phantom: PhantomData<TArithmeticTraits>,
}
impl<TArithmeticTraits: WrappingArithmeticTraits> OperationLibFunc<TArithmeticTraits> {
    fn new(operator: Operator) -> Self {
        Self { operator, _phantom: PhantomData::<TArithmeticTraits>::default() }
    }
}
impl<TArithmeticTraits: WrappingArithmeticTraits> GenericLibFunc
    for OperationLibFunc<TArithmeticTraits>
{
    type Concrete = OperationConcreteLibFunc;

    fn by_id(id: &GenericLibFuncId) -> Option<Self> {
        match id {
            id if id == &TArithmeticTraits::ADD => Some(Self::new(Operator::Add)),
            id if id == &TArithmeticTraits::SUB => Some(Self::new(Operator::Sub)),
            id if id == &TArithmeticTraits::MUL => Some(Self::new(Operator::Mul)),
            id if id == &TArithmeticTraits::DIV => Some(Self::new(Operator::Div)),
            id if id == &TArithmeticTraits::MOD => Some(Self::new(Operator::Mod)),
            _ => None,
        }
    }

    fn specialize_signature(
        &self,
        context: &dyn SignatureSpecializationContext,
        args: &[GenericArg],
    ) -> Result<LibFuncSignature, SpecializationError> {
        let ty = context.get_concrete_type(TArithmeticTraits::GENERIC_TYPE_ID, &[])?;
        match args {
            [] => Ok(LibFuncSignature::new_non_branch(
                vec![
                    ty.clone(),
                    if matches!(self.operator, Operator::Div | Operator::Mod) {
                        context.get_wrapped_concrete_type(NonZeroType::id(), ty.clone())?
                    } else {
                        ty.clone()
                    },
                ],
                vec![OutputVarInfo {
                    ty,
                    ref_info: OutputVarReferenceInfo::Deferred(DeferredOutputKind::Generic),
                }],
                SierraApChange::NotImplemented,
            )),
            [GenericArg::Value(c)] => {
                if matches!(self.operator, Operator::Div | Operator::Mod) && *c == 0 {
                    Err(SpecializationError::UnsupportedGenericArg)
                } else {
                    Ok(LibFuncSignature::new_non_branch(
                        vec![ty.clone()],
                        vec![OutputVarInfo {
                            ty,
                            ref_info: OutputVarReferenceInfo::Deferred(DeferredOutputKind::Generic),
                        }],
                        SierraApChange::NotImplemented,
                    ))
                }
            }
            _ => Err(SpecializationError::UnsupportedGenericArg),
        }
    }

    fn specialize(
        &self,
        context: &dyn SpecializationContext,
        args: &[GenericArg],
    ) -> Result<Self::Concrete, SpecializationError> {
        match args {
            [] => Ok(OperationConcreteLibFunc::Binary(BinaryOperationConcreteLibFunc {
                operator: self.operator,
                signature: self.specialize_signature(context.upcast(), args)?,
            })),
            [GenericArg::Value(c)] => {
                if matches!(self.operator, Operator::Div | Operator::Mod) && *c == 0 {
                    Err(SpecializationError::UnsupportedGenericArg)
                } else {
                    Ok(OperationConcreteLibFunc::Const(OperationWithConstConcreteLibFunc {
                        operator: self.operator,
                        c: *c,
                        signature: self.specialize_signature(context.upcast(), args)?,
                    }))
                }
            }
            _ => Err(SpecializationError::UnsupportedGenericArg),
        }
    }
}

pub struct BinaryOperationConcreteLibFunc {
    pub operator: Operator,
    pub signature: LibFuncSignature,
}
impl SignatureBasedConcreteLibFunc for BinaryOperationConcreteLibFunc {
    fn signature(&self) -> &LibFuncSignature {
        &self.signature
    }
}

/// Arithmetic operations with a const.
pub struct OperationWithConstConcreteLibFunc {
    pub operator: Operator,
    pub c: i64,
    pub signature: LibFuncSignature,
}
define_concrete_libfunc_hierarchy! {
    pub enum OperationConcreteLibFunc {
        Binary(BinaryOperationConcreteLibFunc),
        Const(OperationWithConstConcreteLibFunc),
    }
}

impl SignatureBasedConcreteLibFunc for OperationWithConstConcreteLibFunc {
    fn signature(&self) -> &LibFuncSignature {
        &self.signature
    }
}

/// LibFunc for creating a constant.
#[derive(Default)]
pub struct ConstLibFunc<TArithmeticTraits: WrappingArithmeticTraits> {
    _phantom: PhantomData<TArithmeticTraits>,
}
impl<TArithmeticTraits: WrappingArithmeticTraits> NamedLibFunc for ConstLibFunc<TArithmeticTraits> {
    type Concrete = ConstConcreteLibFunc;
    const ID: GenericLibFuncId = TArithmeticTraits::CONST;

    fn specialize_signature(
        &self,
        context: &dyn SignatureSpecializationContext,
        _args: &[GenericArg],
    ) -> Result<LibFuncSignature, SpecializationError> {
        Ok(LibFuncSignature::new_non_branch(
            vec![],
            vec![OutputVarInfo {
                ty: context.get_concrete_type(TArithmeticTraits::GENERIC_TYPE_ID, &[])?,
                ref_info: OutputVarReferenceInfo::Const,
            }],
            SierraApChange::NotImplemented,
        ))
    }

    fn specialize(
        &self,
        context: &dyn SpecializationContext,
        args: &[GenericArg],
    ) -> Result<Self::Concrete, SpecializationError> {
        match args {
            [GenericArg::Value(c)] => Ok(ConstConcreteLibFunc {
                c: *c,
                signature: <Self as NamedLibFunc>::specialize_signature(
                    self,
                    context.upcast(),
                    args,
                )?,
            }),
            _ => Err(SpecializationError::UnsupportedGenericArg),
        }
    }
}

pub struct ConstConcreteLibFunc {
    pub c: i64,
    pub signature: LibFuncSignature,
}
impl SignatureBasedConcreteLibFunc for ConstConcreteLibFunc {
    fn signature(&self) -> &LibFuncSignature {
        &self.signature
    }
}