use std::{ffi::CStr, os::raw::c_char};

use flecs_ecs_sys::ecs_add_id;

use crate::{
    core::{
        c_types::{EntityT, IdT, InOutKind, IterT, OperKind, WorldT, ECS_DEPENDS_ON, ECS_PAIR},
        component_registration::ComponentInfo,
        utility::errors::FlecsErrorCode,
        RUST_ECS_ID_FLAGS_MASK,
    },
    ecs_assert,
    sys::{
        ecs_field_w_size, ecs_get_mut_id, ecs_get_mut_modified_id, ecs_has_id, ecs_is_deferred,
        ecs_modified_id, ecs_os_api, ecs_strip_generation, ECS_GENERATION_MASK, ECS_ROW_MASK,
    },
};

use super::{
    super::c_types::RUST_ECS_COMPONENT_MASK,
    traits::{InOutType, OperType},
};

/// Combines two 32 bit integers into a 64 bit integer.
///
/// # Arguments
///
/// * `lo`: The lower 32 bit integer.
/// * `hi`: The higher 32 bit integer.
///
/// # Returns
///
/// The combined 64 bit integer.
#[inline(always)]
pub fn ecs_entity_t_comb(lo: u64, hi: u64) -> u64 {
    (hi << 32) + lo
}

/// Combines two 32 bit integers into a 64 bit integer and adds the `ECS_PAIR` flag.
///
/// # Arguments
///
/// * `pred`: The first 32 bit integer.
/// * `obj`: The second 32 bit integer.
///
/// # Returns
///
/// The combined 64 bit integer with the `ECS_PAIR` flag set.
#[inline(always)]
pub fn ecs_pair(pred: u64, obj: u64) -> u64 {
    ECS_PAIR | ecs_entity_t_comb(obj, pred)
}

/// Checks if given entity is a pair
pub fn ecs_is_pair(entity: EntityT) -> bool {
    entity & RUST_ECS_ID_FLAGS_MASK == ECS_PAIR
}

/// Set the `ECS_DEPENDS_ON` flag for the given entity.
///
/// # Arguments
///
/// * `entity`: The entity to set the `ECS_DEPENDS_ON` flag for.
///
/// # Returns
///
/// The entity with the `ECS_DEPENDS_ON` flag set.
#[inline(always)]
pub fn ecs_dependson(entity: EntityT) -> EntityT {
    ecs_pair(ECS_DEPENDS_ON, entity)
}

/// Returns true if the entity has the given pair.
///
/// # Arguments
///
/// * `world`: The world to check in.
/// * `entity`: The entity to check.
/// * `first`: The first entity of the pair.
/// * `second`: The second entity of the pair.
///
/// # Returns
///
/// True if the entity has the given pair, false otherwise.
#[allow(clippy::not_unsafe_ptr_arg_deref)]
#[inline(always)]
pub fn ecs_has_pair(world: *const WorldT, entity: u64, first: u64, second: u64) -> bool {
    unsafe { ecs_has_id(world, entity, ecs_pair(first, second)) }
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
#[inline(always)]
pub fn ecs_add_pair(world: *mut WorldT, entity: u64, first: u64, second: u64) {
    unsafe { ecs_add_id(world, entity, ecs_pair(first, second)) };
}

/// Get the first entity from a pair.
///
/// # Arguments
///
/// * `e`: The pair to get the first entity from.
///
/// # Returns
///
/// The first entity from the pair.
#[inline(always)]
pub fn ecs_pair_first(e: u64) -> u64 {
    ecs_entity_t_hi(e & RUST_ECS_COMPONENT_MASK)
}

/// Get the second entity from a pair.
///
/// # Arguments
///
/// * `e`: The pair to get the second entity from.
///
/// # Returns
///
/// The second entity from the pair.
#[inline(always)]
pub fn ecs_pair_second(e: u64) -> u64 {
    ecs_entity_t_lo(e)
}

/// Get the lower 32 bits of an entity id.
///
/// # Arguments
///
/// * `value`: The entity id to get the lower 32 bits from.
///
/// # Returns
///
/// The lower 32 bits of the entity id.
#[inline(always)]
pub fn ecs_entity_t_lo(value: u64) -> u64 {
    value as u32 as u64
}

/// Get the higher 32 bits of an entity id.
///
/// # Arguments
///
/// * `value`: The entity id to get the higher 32 bits from.
///
/// # Returns
///
/// The higher 32 bits of the entity id.
#[inline(always)]
pub fn ecs_entity_t_hi(value: u64) -> u64 {
    value >> 32
}

/// Get the type name of the given type.
///
/// # Type Parameters
///
/// * `T`: The type to get the name of.
///
/// # Returns
///
/// `[Type]` string slice.
///
/// # Example
///
/// ```
/// use flecs_ecs::core::get_only_type_name;
///
/// pub mod Bar {
///     pub struct Foo;
/// }
///
/// let name = get_only_type_name::<Bar::Foo>();
/// assert_eq!(name, "Foo");
/// ```
#[inline(always)]
pub fn get_only_type_name<T>() -> &'static str {
    use std::any::type_name;
    let name = type_name::<T>();
    name.split("::").last().unwrap_or(name)
}

/// Get the full type name of the given type.
///
/// # Type Parameters
///
/// * `T`: The type to get the name of.
///
/// # Returns
///
/// `[module]::[type]` string slice.
///
/// # Example
///
/// ```rust,ignore
///
/// pub mod Bar {
///     pub struct Foo;
/// }
///
/// let name = get_full_type_name::<Bar::Foo>();
/// assert_eq!(name, "Bar::Foo");
/// ```
#[inline(always)]
pub fn get_full_type_name<T>() -> &'static str {
    use std::any::type_name;
    type_name::<T>()
}

/// Returns true if the given type is an empty type.
///
/// # Type Parameters
///
/// * `T`: The type to check.
#[inline(always)]
pub fn is_empty_type<T>() -> bool {
    std::mem::size_of::<T>() == 0
}

/// Extracts a row index from an ECS record identifier.
///
/// Applies a bitwise AND with `ECS_ROW_MASK` to `row`, isolating relevant bits,
/// and returns the result as an `i32`. This function is typically used to decode
/// information about an entity's components or position encoded within the record identifier.
///
/// # Arguments
///
/// * `row`: A `u32` representing an ECS record identifier.
///
/// # Returns
///
/// * `i32`: The decoded row index.
pub fn ecs_record_to_row(row: u32) -> i32 {
    (row & ECS_ROW_MASK) as i32
}

/// Internal helper function to set a component for an entity.
///
/// This function sets the given value for an entity in the ECS world, ensuring
/// that the type of the component is valid.
///
/// # Type Parameters
///
/// * `T`: The type of the component data. Must implement `ComponentInfo`.
///
/// # Arguments
///
/// * `entity`: The ID of the entity.
/// * `value`: The value to set for the component.
/// * `id`: The ID of the component type.
pub(crate) fn set_helper<T: ComponentInfo>(world: *mut WorldT, entity: EntityT, value: T, id: IdT) {
    ecs_assert!(
        T::get_size(world) != 0,
        FlecsErrorCode::InvalidParameter,
        "invalid type: {}",
        T::get_symbol_name()
    );
    unsafe {
        if !ecs_is_deferred(world) {
            let comp = ecs_get_mut_id(world, entity, id) as *mut T;

            std::ptr::write(comp, value);
            ecs_modified_id(world, entity, id);
        } else {
            let comp = ecs_get_mut_modified_id(world, entity, id) as *mut T;
            std::ptr::write(comp, value);
            ecs_modified_id(world, entity, id);
        }
    }
}

/// Remove generation from entity id.
///
/// # Arguments
///
/// * `entity`: The entity id to strip the generation from.
///
/// # Returns
///
/// * `IdT`: The entity id with the generation removed.
#[inline(always)]
pub fn strip_generation(entity: EntityT) -> IdT {
    unsafe { ecs_strip_generation(entity) }
}

/// Get the generation from an entity id.
///
/// # Arguments
///
/// * `entity`: The entity id to get the generation from.
///
/// # Returns
///
/// * `u32`: The generation of the entity id.
#[inline(always)]
pub fn get_generation(entity: EntityT) -> u32 {
    ((entity & ECS_GENERATION_MASK) >> 32) as u32
}

/// Gets the component data from the iterator.
/// Retrieves a pointer to the data array for a specified query field.
///
/// This function obtains a pointer to an array of data corresponding to the term in the query,
/// based on the given index. The index starts from 1, representing the first term in the query.
///
/// For instance, given a query "Position, Velocity", invoking this function with index 1 would
/// return a pointer to the "Position" data array, and index 2 would return the "Velocity" data array.
///
/// If the specified field is not owned by the entity being iterated (e.g., a shared component from a prefab,
/// a component from a parent, or a component from another entity), this function returns a direct pointer
/// instead of an array pointer. Use `ecs_field_is_self` to dynamically check if a field is owned.
///
/// The `size` of the type `T` must match the size of the data type of the returned array. Mismatches between
/// the provided type size and the actual data type size may cause the operation to assert. The size of the
/// field can be obtained dynamically using `ecs_field_size`.
///
/// # Safety
///
/// This function is unsafe because it dereferences the iterator and uses the index to get the component data.
/// Ensure that the iterator is valid and the index is valid.
///
/// # Arguments
///
/// - `it`: A pointer to the iterator.
/// - `index`: The index of the field in the iterator, starting from 1.
///
/// # Returns
///
/// A pointer to the data of the specified field. The pointer type is determined by the generic type `T`.
///
/// # Example
///
/// ```rust,ignore
/// // Assuming `it` is a valid iterator pointer obtained from a query.
/// let position_ptr: *mut Position = ecs_field(it, 1);
/// let velocity_ptr: *mut Velocity = ecs_field(it, 2);
/// ```
#[inline(always)]
pub unsafe fn ecs_field<T: ComponentInfo>(it: *const IterT, index: i32) -> *mut T {
    let size = std::mem::size_of::<T>();
    ecs_field_w_size(it, size, index) as *mut T
}

/// Get the `InOutKind` for the given type.
///
/// # Type Parameters
///
/// * `T`: The type to get the `InOutKind` for.
///
/// # See also
///
/// * C++ API: `type_to_inout`
pub(crate) fn type_to_inout<T: InOutType>() -> InOutKind {
    T::IN_OUT
}

/// Get the `OperKind` for the given type.
///
/// # Type Parameters
///
/// * `T`: The type to get the `OperKind` for.
///
/// # See also
///
/// * C++ API: `type_to_oper`
pub(crate) fn type_to_oper<T: OperType>() -> OperKind {
    T::OPER
}

/// Copies the given Rust &str to a C string and returns a pointer to the C string.
/// this is intended to be used when the C code needs to take ownership of the string.
///
/// # Note
///
/// This function isn't being used anymore and might be removed in the future.
pub(crate) fn copy_and_allocate_c_char_from_rust_str(data: &str) -> *mut c_char {
    ecs_assert!(
        data.is_ascii(),
        FlecsErrorCode::InvalidParameter,
        "string must be ascii"
    );
    let bytes = data.as_bytes();
    let len = bytes.len() + 1; // +1 for the null terminator
    let memory_c_str = unsafe { ecs_os_api.malloc_.unwrap()(len as i32) } as *mut u8;

    for (i, &byte) in bytes.iter().enumerate() {
        unsafe {
            memory_c_str.add(i).write(byte);
        }
    }

    // Write the null terminator to the end of the memory
    unsafe { memory_c_str.add(bytes.len()).write(0) };

    memory_c_str as *mut c_char
}

/// Prints the given C string to the console.
///
/// # Note
///
/// This function is for development purposes. It is not intended to be used in production code.
pub(crate) unsafe fn print_c_string(c_string: *const c_char) {
    // Ensure the pointer is not null
    assert!(!c_string.is_null(), "Null pointer passed to print_c_string");

    // Create a CStr from the raw pointer
    let c_str = std::ffi::CStr::from_ptr(c_string);

    // Convert CStr to a Rust string slice (&str)
    // This can fail if the C string is not valid UTF-8, so handle errors appropriately
    match c_str.to_str() {
        Ok(s) => println!("{}", s),
        Err(_) => println!("Failed to convert C string to Rust string"),
    }
}

/// Strips the given prefix from the given C string, returning a new C string with the prefix removed.
/// If the given C string does not start with the given prefix, returns None.
pub(crate) fn strip_prefix_cstr_raw(cstr: &'static CStr, prefix: &CStr) -> Option<&'static CStr> {
    let cstr_bytes = cstr.to_bytes();
    let prefix_bytes = prefix.to_bytes();

    if cstr_bytes.starts_with(prefix_bytes) {
        // SAFETY: We are slicing `cstr_bytes` which is guaranteed to be a valid
        // C string since it comes from a `&CStr`. We also check that it starts
        // with `prefix_bytes`, and we only slice off `prefix_bytes`, so the rest
        // remains a valid C string.
        unsafe {
            Some(CStr::from_bytes_with_nul_unchecked(
                &cstr_bytes[prefix_bytes.len()..],
            ))
        }
    } else {
        None
    }
}
