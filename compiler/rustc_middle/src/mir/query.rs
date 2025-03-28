//! Values computed by queries that use MIR.

use std::fmt::{self, Debug};

use rustc_abi::{FieldIdx, VariantIdx};
use rustc_data_structures::fx::FxIndexMap;
use rustc_errors::ErrorGuaranteed;
use rustc_hir::def_id::LocalDefId;
use rustc_index::bit_set::BitMatrix;
use rustc_index::{Idx, IndexVec};
use rustc_macros::{HashStable, TyDecodable, TyEncodable, TypeFoldable, TypeVisitable};
use rustc_span::{Span, Symbol};
use smallvec::SmallVec;

use super::{ConstValue, SourceInfo};
use crate::ty::{self, CoroutineArgsExt, OpaqueHiddenType, Ty, TyCtxt, fold_regions};

rustc_index::newtype_index! {
    #[derive(HashStable)]
    #[encodable]
    #[debug_format = "_{}"]
    pub struct CoroutineSavedLocal {}
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[derive(TyEncodable, TyDecodable, HashStable, TypeFoldable, TypeVisitable)]
pub struct CoroutineSavedTy<'tcx> {
    pub ty: Ty<'tcx>,
    /// Source info corresponding to the local in the original MIR body.
    pub source_info: SourceInfo,
    /// Whether the local should be ignored for trait bound computations.
    pub ignore_for_traits: bool,
}

/// The layout of coroutine state.
#[derive(Clone, PartialEq, Eq)]
#[derive(TyEncodable, TyDecodable, HashStable, TypeFoldable, TypeVisitable)]
pub struct CoroutineLayout<'tcx> {
    /// The type of every local stored inside the coroutine.
    pub field_tys: IndexVec<CoroutineSavedLocal, CoroutineSavedTy<'tcx>>,

    /// The name for debuginfo.
    pub field_names: IndexVec<CoroutineSavedLocal, Option<Symbol>>,

    /// Which of the above fields are in each variant. Note that one field may
    /// be stored in multiple variants.
    pub variant_fields: IndexVec<VariantIdx, IndexVec<FieldIdx, CoroutineSavedLocal>>,

    /// The source that led to each variant being created (usually, a yield or
    /// await).
    pub variant_source_info: IndexVec<VariantIdx, SourceInfo>,

    /// Which saved locals are storage-live at the same time. Locals that do not
    /// have conflicts with each other are allowed to overlap in the computed
    /// layout.
    #[type_foldable(identity)]
    #[type_visitable(ignore)]
    pub storage_conflicts: BitMatrix<CoroutineSavedLocal, CoroutineSavedLocal>,
}

impl Debug for CoroutineLayout<'_> {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_struct("CoroutineLayout")
            .field_with("field_tys", |fmt| {
                fmt.debug_map().entries(self.field_tys.iter_enumerated()).finish()
            })
            .field_with("variant_fields", |fmt| {
                let mut map = fmt.debug_map();
                for (idx, fields) in self.variant_fields.iter_enumerated() {
                    map.key_with(|fmt| {
                        let variant_name = ty::CoroutineArgs::variant_name(idx);
                        if fmt.alternate() {
                            write!(fmt, "{variant_name:9}({idx:?})")
                        } else {
                            write!(fmt, "{variant_name}")
                        }
                    });
                    // Force variant fields to print in regular mode instead of alternate mode.
                    map.value_with(|fmt| write!(fmt, "{fields:?}"));
                }
                map.finish()
            })
            .field("storage_conflicts", &self.storage_conflicts)
            .finish()
    }
}

#[derive(Debug, TyEncodable, TyDecodable, HashStable)]
pub struct BorrowCheckResult<'tcx> {
    /// All the opaque types that are restricted to concrete types
    /// by this function. Unlike the value in `TypeckResults`, this has
    /// unerased regions.
    pub concrete_opaque_types: FxIndexMap<LocalDefId, OpaqueHiddenType<'tcx>>,
    pub closure_requirements: Option<ClosureRegionRequirements<'tcx>>,
    pub used_mut_upvars: SmallVec<[FieldIdx; 8]>,
    pub tainted_by_errors: Option<ErrorGuaranteed>,
}

/// The result of the `mir_const_qualif` query.
///
/// Each field (except `tainted_by_errors`) corresponds to an implementer of the `Qualif` trait in
/// `rustc_const_eval/src/transform/check_consts/qualifs.rs`. See that file for more information on each
/// `Qualif`.
#[derive(Clone, Copy, Debug, Default, TyEncodable, TyDecodable, HashStable)]
pub struct ConstQualifs {
    pub has_mut_interior: bool,
    pub needs_drop: bool,
    pub needs_non_const_drop: bool,
    pub tainted_by_errors: Option<ErrorGuaranteed>,
}

/// After we borrow check a closure, we are left with various
/// requirements that we have inferred between the free regions that
/// appear in the closure's signature or on its field types. These
/// requirements are then verified and proved by the closure's
/// creating function. This struct encodes those requirements.
///
/// The requirements are listed as being between various `RegionVid`. The 0th
/// region refers to `'static`; subsequent region vids refer to the free
/// regions that appear in the closure (or coroutine's) type, in order of
/// appearance. (This numbering is actually defined by the `UniversalRegions`
/// struct in the NLL region checker. See for example
/// `UniversalRegions::closure_mapping`.) Note the free regions in the
/// closure's signature and captures are erased.
///
/// Example: If type check produces a closure with the closure args:
///
/// ```text
/// ClosureArgs = [
///     'a,                                         // From the parent.
///     'b,
///     i8,                                         // the "closure kind"
///     for<'x> fn(&'<erased> &'x u32) -> &'x u32,  // the "closure signature"
///     &'<erased> String,                          // some upvar
/// ]
/// ```
///
/// We would "renumber" each free region to a unique vid, as follows:
///
/// ```text
/// ClosureArgs = [
///     '1,                                         // From the parent.
///     '2,
///     i8,                                         // the "closure kind"
///     for<'x> fn(&'3 &'x u32) -> &'x u32,         // the "closure signature"
///     &'4 String,                                 // some upvar
/// ]
/// ```
///
/// Now the code might impose a requirement like `'1: '2`. When an
/// instance of the closure is created, the corresponding free regions
/// can be extracted from its type and constrained to have the given
/// outlives relationship.
#[derive(Clone, Debug, TyEncodable, TyDecodable, HashStable)]
pub struct ClosureRegionRequirements<'tcx> {
    /// The number of external regions defined on the closure. In our
    /// example above, it would be 3 -- one for `'static`, then `'1`
    /// and `'2`. This is just used for a sanity check later on, to
    /// make sure that the number of regions we see at the callsite
    /// matches.
    pub num_external_vids: usize,

    /// Requirements between the various free regions defined in
    /// indices.
    pub outlives_requirements: Vec<ClosureOutlivesRequirement<'tcx>>,
}

/// Indicates an outlives-constraint between a type or between two
/// free regions declared on the closure.
#[derive(Copy, Clone, Debug, TyEncodable, TyDecodable, HashStable)]
pub struct ClosureOutlivesRequirement<'tcx> {
    // This region or type ...
    pub subject: ClosureOutlivesSubject<'tcx>,

    // ... must outlive this one.
    pub outlived_free_region: ty::RegionVid,

    // If not, report an error here ...
    pub blame_span: Span,

    // ... due to this reason.
    pub category: ConstraintCategory<'tcx>,
}

// Make sure this enum doesn't unintentionally grow
#[cfg(target_pointer_width = "64")]
rustc_data_structures::static_assert_size!(ConstraintCategory<'_>, 16);

/// Outlives-constraints can be categorized to determine whether and why they
/// are interesting (for error reporting). Order of variants indicates sort
/// order of the category, thereby influencing diagnostic output.
///
/// See also `rustc_const_eval::borrow_check::constraints`.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
#[derive(TyEncodable, TyDecodable, HashStable, TypeVisitable, TypeFoldable)]
pub enum ConstraintCategory<'tcx> {
    Return(ReturnConstraint),
    Yield,
    UseAsConst,
    UseAsStatic,
    TypeAnnotation(AnnotationSource),
    Cast {
        /// Whether this cast is a coercion that was automatically inserted by the compiler.
        is_implicit_coercion: bool,
        /// Whether this is an unsizing coercion and if yes, this contains the target type.
        /// Region variables are erased to ReErased.
        unsize_to: Option<Ty<'tcx>>,
    },

    /// Contains the function type if available.
    CallArgument(Option<Ty<'tcx>>),
    CopyBound,
    SizedBound,
    Assignment,
    /// A constraint that came from a usage of a variable (e.g. in an ADT expression
    /// like `Foo { field: my_val }`)
    Usage,
    OpaqueType,
    ClosureUpvar(FieldIdx),

    /// A constraint from a user-written predicate
    /// with the provided span, written on the item
    /// with the given `DefId`
    Predicate(Span),

    /// A "boring" constraint (caused by the given location) is one that
    /// the user probably doesn't want to see described in diagnostics,
    /// because it is kind of an artifact of the type system setup.
    Boring,
    // Boring and applicable everywhere.
    BoringNoLocation,

    /// A constraint that doesn't correspond to anything the user sees.
    Internal,

    /// An internal constraint derived from an illegal universe relation.
    IllegalUniverse,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
#[derive(TyEncodable, TyDecodable, HashStable, TypeVisitable, TypeFoldable)]
pub enum ReturnConstraint {
    Normal,
    ClosureUpvar(FieldIdx),
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
#[derive(TyEncodable, TyDecodable, HashStable, TypeVisitable, TypeFoldable)]
pub enum AnnotationSource {
    Ascription,
    Declaration,
    OpaqueCast,
    GenericArg,
}

/// The subject of a `ClosureOutlivesRequirement` -- that is, the thing
/// that must outlive some region.
#[derive(Copy, Clone, Debug, TyEncodable, TyDecodable, HashStable)]
pub enum ClosureOutlivesSubject<'tcx> {
    /// Subject is a type, typically a type parameter, but could also
    /// be a projection. Indicates a requirement like `T: 'a` being
    /// passed to the caller, where the type here is `T`.
    Ty(ClosureOutlivesSubjectTy<'tcx>),

    /// Subject is a free region from the closure. Indicates a requirement
    /// like `'a: 'b` being passed to the caller; the region here is `'a`.
    Region(ty::RegionVid),
}

/// Represents a `ty::Ty` for use in [`ClosureOutlivesSubject`].
///
/// This abstraction is necessary because the type may include `ReVar` regions,
/// which is what we use internally within NLL code, and they can't be used in
/// a query response.
///
/// DO NOT implement `TypeVisitable` or `TypeFoldable` traits, because this
/// type is not recognized as a binder for late-bound region.
#[derive(Copy, Clone, Debug, TyEncodable, TyDecodable, HashStable)]
pub struct ClosureOutlivesSubjectTy<'tcx> {
    inner: Ty<'tcx>,
}

impl<'tcx> ClosureOutlivesSubjectTy<'tcx> {
    /// All regions of `ty` must be of kind `ReVar` and must represent
    /// universal regions *external* to the closure.
    pub fn bind(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> Self {
        let inner = fold_regions(tcx, ty, |r, depth| match r.kind() {
            ty::ReVar(vid) => {
                let br = ty::BoundRegion {
                    var: ty::BoundVar::new(vid.index()),
                    kind: ty::BoundRegionKind::Anon,
                };
                ty::Region::new_bound(tcx, depth, br)
            }
            _ => bug!("unexpected region in ClosureOutlivesSubjectTy: {r:?}"),
        });

        Self { inner }
    }

    pub fn instantiate(
        self,
        tcx: TyCtxt<'tcx>,
        mut map: impl FnMut(ty::RegionVid) -> ty::Region<'tcx>,
    ) -> Ty<'tcx> {
        fold_regions(tcx, self.inner, |r, depth| match r.kind() {
            ty::ReBound(debruijn, br) => {
                debug_assert_eq!(debruijn, depth);
                map(ty::RegionVid::new(br.var.index()))
            }
            _ => bug!("unexpected region {r:?}"),
        })
    }
}

/// The constituent parts of a mir constant of kind ADT or array.
#[derive(Copy, Clone, Debug, HashStable)]
pub struct DestructuredConstant<'tcx> {
    pub variant: Option<VariantIdx>,
    pub fields: &'tcx [(ConstValue<'tcx>, Ty<'tcx>)],
}
