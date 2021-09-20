#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{
    ensure,
    decl_error, 
    decl_module, 
    decl_storage,
    dispatch::{
        DispatchResult, 
        DispatchError, 
        Vec,
    },
};
use frame_system::{
    ensure_signed,
};

use frame_support::sp_std::{
    cmp::{
        Eq, 
        PartialEq}, 
};

use file::{FileStruct, H256};

#[cfg(test)]
mod mock;

#[cfg(test)]    
mod tests;
mod file;

pub trait Config: frame_system::Config {}

decl_storage! {
    trait Store for Module<T: Config> as Audit {
        /// Storage map for file IDs
        FileByID
            get(fn file_by_id):
            map hasher(blake2_128_concat) u32 => FileStruct<T::AccountId>;   

        /// Last Id of created file
        LastID: u32;
    }
}

decl_error! {
    pub enum Error for Module<T: Config> {
        AddressNotAuditor,
        AddressNotOwner
    }
}

decl_module! {
    pub struct Module<T: Config> for enum Call where origin: T::Origin {
        #[weight = 10_000]
		pub fn sign_latest_version(origin, id: u32) {
			let caller = ensure_signed(origin)?;
            ensure!(Self::address_is_auditor_for_file(id, &caller), Error::<T>::AddressNotAuditor);
            FileByID::<T>::try_mutate(
                id, |file_by_id| -> DispatchResult {
                    file_by_id.sign_latest_version(caller);
                    Ok(())
                })?;
		}

        #[weight = 10_000]
        pub fn create_new_file(origin, tag: Vec<u8>, filehash: H256) -> DispatchResult {
            if tag.len() == 0 {
                return Err(DispatchError::Other("empty input file"))
            }
            let caller = ensure_signed(origin)?;
            
            // Update last created file ID
            let new_id = LastID::get() + 1;
            let new_file = FileStruct::<<T as frame_system::Config>::AccountId>::new(caller, new_id, tag, &filehash);

            <FileByID<T>>::insert(new_id, new_file);
            LastID::mutate(|x| *x += 1);
            Ok(())
        }
        
        #[weight = 10_000]
        pub fn delete_auditor(origin, id: u32, auditor: T::AccountId)  {
            let caller = ensure_signed(origin)?;
            ensure!(Self::address_is_owner_for_file(id, &caller), Error::<T>::AddressNotOwner);

            FileByID::<T>::try_mutate(
                id, |file_by_id| -> DispatchResult {
                    if let Err(_) = file_by_id.delete_auditor_from_file(auditor) {
                        return Err(DispatchError::Other("no auditor"));
                    }
                    Ok(())
                }
            )?;
        }

        #[weight = 10_000]
        pub fn assign_auditor(origin, id: u32, auditor: T::AccountId) {
            let caller = ensure_signed(origin)?;
            ensure!(Self::address_is_owner_for_file(id, &caller), Error::<T>::AddressNotOwner);

            FileByID::<T>::try_mutate(
                id, |file_by_id| -> DispatchResult {
                    file_by_id.assign_auditor_to_file(auditor);
                    Ok(())
                }
            )?;
        }
    }
}

impl<T: Config> Module<T> {
    /// <pre>
    /// Method: address_is_auditor_for_file(id: u32, address: &T::AccountId) -> bool
    /// Arguments: id: u32, address: &T::AccountId - file ID, address
    ///
    /// Checks if the address is an auditor for the given file
    /// </pre>
    pub fn address_is_auditor_for_file(id: u32, address: &T::AccountId) -> bool {
        FileByID::<T>::get(id).auditors.iter().any(|x| x == address)
    }

    /// <pre>
    /// Method: address_is_owner_for_file(id: u32, address: &T::AccountId) -> bool
    /// Arguments: id: u32, address: &T::AccountId - file ID, address
    ///
    /// Checks if the address is the owner for the given file
    /// </pre>
    pub fn address_is_owner_for_file(id: u32, address: &T::AccountId) -> bool {
        FileByID::<T>::get(id).owner == *address
    }

    #[cfg(test)]
    fn get_file_by_id(id: u32) -> FileStruct<<T as frame_system::Config>::AccountId> {
        FileByID::<T>::get(id)
    }
}