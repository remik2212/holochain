use crate::{HashType, HoloHash};
use holochain_serialized_bytes::{SerializedBytes, SerializedBytesError, UnsafeBytes};

impl<T: HashType> std::convert::TryFrom<&HoloHash<T>> for SerializedBytes {
    type Error = SerializedBytesError;
    fn try_from(t: &HoloHash<T>) -> std::result::Result<SerializedBytes, SerializedBytesError> {
        match rmp_serde::to_vec_named(t) {
            Ok(v) => Ok(SerializedBytes::from(UnsafeBytes::from(v))),
            Err(e) => Err(SerializedBytesError::ToBytes(e.to_string())),
        }
    }
}

impl<T: HashType> std::convert::TryFrom<HoloHash<T>> for SerializedBytes {
    type Error = SerializedBytesError;
    fn try_from(t: HoloHash<T>) -> std::result::Result<SerializedBytes, SerializedBytesError> {
        SerializedBytes::try_from(&t)
    }
}

impl<T: HashType> std::convert::TryFrom<SerializedBytes> for HoloHash<T> {
    type Error = SerializedBytesError;
    fn try_from(sb: SerializedBytes) -> std::result::Result<HoloHash<T>, SerializedBytesError> {
        match rmp_serde::from_read_ref(sb.bytes()) {
            Ok(v) => Ok(v),
            Err(e) => Err(SerializedBytesError::FromBytes(e.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::*;

    #[test]
    #[cfg(feature = "serialized-bytes")]
    fn test_serialized_bytes_roundtrip() {
        use holochain_serialized_bytes::SerializedBytes;
        use std::convert::TryInto;

        let h_orig = DnaHash::from_raw_bytes(vec![0xdb; 36]);
        let h: SerializedBytes = h_orig.clone().try_into().unwrap();
        let h: DnaHash = h.try_into().unwrap();

        assert_eq!(h_orig, h);
        assert_eq!(*h.hash_type(), hash_type::Dna::new());
    }

    #[test]
    fn test_rmp_roundtrip() {
        let h_orig = AgentPubKey::from_raw_bytes(vec![0xdb; 36]);
        let buf = rmp_serde::to_vec_named(&h_orig).unwrap();
        let h: AgentPubKey = rmp_serde::from_read_ref(&buf).unwrap();

        assert_eq!(h_orig, h);
        assert_eq!(*h.hash_type(), hash_type::Agent::new());
    }

    #[test]
    fn test_composite_hashtype_roundtrips() {
        {
            let h_orig =
                EntryHash::from_raw_bytes_and_type(vec![0xdb; 36], hash_type::Entry::Content);
            let buf = rmp_serde::to_vec_named(&h_orig).unwrap();
            let h: EntryHash = rmp_serde::from_read_ref(&buf).unwrap();
            assert_eq!(h_orig, h);
            assert_eq!(*h.hash_type(), hash_type::Entry::Content);
        }
        {
            let h_orig =
                EntryHash::from_raw_bytes_and_type(vec![0xdb; 36], hash_type::Entry::Agent);
            let buf = rmp_serde::to_vec_named(&h_orig).unwrap();
            let h: EntryHash = rmp_serde::from_read_ref(&buf).unwrap();
            assert_eq!(h_orig, h);
            assert_eq!(*h.hash_type(), hash_type::Entry::Agent);
        }
        {
            let h_orig =
                AnyDhtHash::from_raw_bytes_and_type(vec![0xdb; 36], hash_type::AnyDht::Header);
            let buf = rmp_serde::to_vec_named(&h_orig).unwrap();
            let h: AnyDhtHash = rmp_serde::from_read_ref(&buf).unwrap();
            assert_eq!(h_orig, h);
            assert_eq!(*h.hash_type(), hash_type::AnyDht::Header);
        }
        {
            let h_orig = AnyDhtHash::from_raw_bytes_and_type(
                vec![0xdb; 36],
                hash_type::AnyDht::Entry(hash_type::Entry::Content),
            );
            let buf = rmp_serde::to_vec_named(&h_orig).unwrap();
            let h: AnyDhtHash = rmp_serde::from_read_ref(&buf).unwrap();
            assert_eq!(h_orig, h);
            assert_eq!(
                *h.hash_type(),
                hash_type::AnyDht::Entry(hash_type::Entry::Content)
            );
        }
        {
            let h_orig = AnyDhtHash::from_raw_bytes_and_type(
                vec![0xdb; 36],
                hash_type::AnyDht::Entry(hash_type::Entry::Agent),
            );
            let buf = rmp_serde::to_vec_named(&h_orig).unwrap();
            let h: AnyDhtHash = rmp_serde::from_read_ref(&buf).unwrap();
            assert_eq!(h_orig, h);
            assert_eq!(
                *h.hash_type(),
                hash_type::AnyDht::Entry(hash_type::Entry::Agent)
            );
        }
    }

    #[test]
    #[should_panic]
    fn test_composite_hashtype_crossover_error_1() {
        {
            let h_orig =
                EntryHash::from_raw_bytes_and_type(vec![0xdb; 36], hash_type::Entry::Content);
            let buf = rmp_serde::to_vec_named(&h_orig).unwrap();
            let _: AnyDhtHash = rmp_serde::from_read_ref(&buf).unwrap();
        }
    }

    #[test]
    #[should_panic]
    fn test_composite_hashtype_crossover_error_2() {
        {
            let h_orig =
                EntryHash::from_raw_bytes_and_type(vec![0xdb; 36], hash_type::Entry::Agent);
            let buf = rmp_serde::to_vec_named(&h_orig).unwrap();
            let _: AnyDhtHash = rmp_serde::from_read_ref(&buf).unwrap();
        }
    }

    #[test]
    #[should_panic]
    fn test_composite_hashtype_crossover_error_3() {
        {
            let h_orig = AnyDhtHash::from_raw_bytes_and_type(
                vec![0xdb; 36],
                hash_type::AnyDht::Entry(hash_type::Entry::Content),
            );
            let buf = rmp_serde::to_vec_named(&h_orig).unwrap();
            let _: EntryHash = rmp_serde::from_read_ref(&buf).unwrap();
        }
    }

    #[test]
    #[should_panic]
    fn test_composite_hashtype_crossover_error_4() {
        {
            let h_orig = AnyDhtHash::from_raw_bytes_and_type(
                vec![0xdb; 36],
                hash_type::AnyDht::Entry(hash_type::Entry::Agent),
            );
            let buf = rmp_serde::to_vec_named(&h_orig).unwrap();
            let _: EntryHash = rmp_serde::from_read_ref(&buf).unwrap();
        }
    }

    #[test]
    #[should_panic]
    fn test_composite_hashtype_crossover_error_5() {
        {
            let h_orig =
                AnyDhtHash::from_raw_bytes_and_type(vec![0xdb; 36], hash_type::AnyDht::Header);
            let buf = rmp_serde::to_vec_named(&h_orig).unwrap();
            let _: EntryHash = rmp_serde::from_read_ref(&buf).unwrap();
        }
    }

    #[test]
    fn test_struct_roundtrip() {
        #[derive(Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
        struct Data {
            a: AgentPubKey,
            h: HeaderHash,
        }

        let orig = Data {
            a: AgentPubKey::from_raw_bytes(vec![0xdb; 36]),
            h: HeaderHash::from_raw_bytes(vec![0xdb; 36]),
        };

        let buf = rmp_serde::to_vec_named(&orig).unwrap();
        let res: Data = rmp_serde::from_read_ref(&buf).unwrap();

        assert_eq!(orig, res);
        assert_eq!(*orig.a.hash_type(), hash_type::Agent::new());
        assert_eq!(*orig.h.hash_type(), hash_type::Header::new());
    }

    #[test]
    fn test_generic_content_roundtrip() {
        #[derive(Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
        struct Generic<K> {
            bytes: Vec<u8>,
            __marker: std::marker::PhantomData<K>,
        }

        impl<K> Generic<K>
        where
            K: serde::Serialize + serde::de::DeserializeOwned + std::fmt::Debug,
            // V: Serialize + DeserializeOwned + std::fmt::Debug,
        {
            fn new() -> Self {
                Self {
                    bytes: Vec::new(),
                    __marker: Default::default(),
                }
            }

            fn get(&self) -> K {
                rmp_serde::from_read_ref(&self.bytes).unwrap()
            }

            fn put(&mut self, k: &K) {
                self.bytes = rmp_serde::to_vec_named(k).unwrap();
            }
        }

        let mut g: Generic<HeaderHash> = Generic::new();
        let h = HeaderHash::from_raw_bytes(vec![0xdb; 36]);
        g.put(&h);
        assert_eq!(h, g.get());
    }
}