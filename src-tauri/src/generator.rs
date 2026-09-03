use std::{error::Error, fmt};

use zeroize::{Zeroize, Zeroizing};

const LOWERCASE: &str = "abcdefghijklmnopqrstuvwxyz";
const UPPERCASE: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";
const NUMBERS: &str = "0123456789";
const SYMBOLS: &str = "!@#$%^&*_-+=?";
const AMBIGUOUS: &str = "Il1O0o5S8B";

const MIN_COUNT: usize = 1;
const MAX_COUNT: usize = 99;
const MIN_LENGTH: usize = 4;
const MAX_LENGTH: usize = 64;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GenerationOptions {
    pub lowercase: bool,
    pub uppercase: bool,
    pub numbers: bool,
    pub symbols: bool,
    pub exclude_ambiguous: bool,
}

#[derive(Debug)]
pub enum GenerateError {
    CountOutOfRange { count: usize },
    LengthOutOfRange { length: usize },
    NoCharacterGroups,
    Entropy(getrandom::Error),
}

impl fmt::Display for GenerateError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CountOutOfRange { count } => {
                write!(formatter, "count must be between 1 and 99; got {count}")
            }
            Self::LengthOutOfRange { length } => {
                write!(formatter, "length must be between 4 and 64; got {length}")
            }
            Self::NoCharacterGroups => formatter.write_str("select at least one character group"),
            Self::Entropy(error) => write!(formatter, "OS entropy unavailable: {error}"),
        }
    }
}

impl Error for GenerateError {}

/// Generates `count` passwords with OS entropy.
///
/// Validation happens before entropy is requested. No fallback RNG is used.
pub fn generate_passwords(
    count: usize,
    length: usize,
    options: GenerationOptions,
) -> Result<Vec<String>, GenerateError> {
    let mut random = OsRandom;
    generate_passwords_with_source(count, length, options, &mut random).map_err(GenerateError::from)
}

trait RandomSource {
    type Error;

    fn fill(&mut self, destination: &mut [u8]) -> Result<(), Self::Error>;
}

struct OsRandom;

impl RandomSource for OsRandom {
    type Error = getrandom::Error;

    fn fill(&mut self, destination: &mut [u8]) -> Result<(), Self::Error> {
        getrandom::fill(destination)
    }
}

#[derive(Debug, Eq, PartialEq)]
enum InternalError<E> {
    CountOutOfRange { count: usize },
    LengthOutOfRange { length: usize },
    NoCharacterGroups,
    Entropy(E),
}

impl From<InternalError<getrandom::Error>> for GenerateError {
    fn from(error: InternalError<getrandom::Error>) -> Self {
        match error {
            InternalError::CountOutOfRange { count } => Self::CountOutOfRange { count },
            InternalError::LengthOutOfRange { length } => Self::LengthOutOfRange { length },
            InternalError::NoCharacterGroups => Self::NoCharacterGroups,
            InternalError::Entropy(error) => Self::Entropy(error),
        }
    }
}

fn generate_passwords_with_source<R: RandomSource>(
    count: usize,
    length: usize,
    options: GenerationOptions,
    random: &mut R,
) -> Result<Vec<String>, InternalError<R::Error>> {
    if !(MIN_COUNT..=MAX_COUNT).contains(&count) {
        return Err(InternalError::CountOutOfRange { count });
    }
    if !(MIN_LENGTH..=MAX_LENGTH).contains(&length) {
        return Err(InternalError::LengthOutOfRange { length });
    }

    let groups = build_groups(options);
    if groups.is_empty() {
        return Err(InternalError::NoCharacterGroups);
    }

    let pool: Vec<u8> = groups.iter().flatten().copied().collect();
    let mut passwords: Vec<String> = Vec::with_capacity(count);

    for _ in 0..count {
        match generate_one(length, &groups, &pool, random) {
            Ok(password) => passwords.push(password),
            Err(error) => {
                for password in &mut passwords {
                    password.zeroize();
                }
                return Err(InternalError::Entropy(error));
            }
        }
    }

    Ok(passwords)
}

fn build_groups(options: GenerationOptions) -> Vec<Vec<u8>> {
    let mut groups = Vec::with_capacity(4);
    add_group(
        &mut groups,
        options.lowercase,
        LOWERCASE,
        options.exclude_ambiguous,
    );
    add_group(
        &mut groups,
        options.uppercase,
        UPPERCASE,
        options.exclude_ambiguous,
    );
    add_group(
        &mut groups,
        options.numbers,
        NUMBERS,
        options.exclude_ambiguous,
    );
    add_group(
        &mut groups,
        options.symbols,
        SYMBOLS,
        options.exclude_ambiguous,
    );
    groups
}

fn add_group(groups: &mut Vec<Vec<u8>>, enabled: bool, characters: &str, exclude_ambiguous: bool) {
    if !enabled {
        return;
    }

    let group = if exclude_ambiguous {
        characters
            .bytes()
            .filter(|character| !AMBIGUOUS.as_bytes().contains(character))
            .collect()
    } else {
        characters.as_bytes().to_vec()
    };
    groups.push(group);
}

fn generate_one<R: RandomSource>(
    length: usize,
    groups: &[Vec<u8>],
    pool: &[u8],
    random: &mut R,
) -> Result<String, R::Error> {
    let mut candidate = Zeroizing::new(vec![0_u8; length]);
    let mut position = 0;

    for group in groups {
        candidate[position] = group[next_index(group.len(), random)?];
        position += 1;
    }
    while position < candidate.len() {
        candidate[position] = pool[next_index(pool.len(), random)?];
        position += 1;
    }

    fisher_yates(&mut candidate, random)?;

    Ok(String::from_utf8(candidate.as_slice().to_vec())
        .expect("LocalPass character groups must remain ASCII"))
}

fn fisher_yates<R: RandomSource>(candidate: &mut [u8], random: &mut R) -> Result<(), R::Error> {
    for index in (1..candidate.len()).rev() {
        let swap = next_index(index + 1, random)?;
        candidate.swap(index, swap);
    }
    Ok(())
}

fn next_index<R: RandomSource>(
    exclusive_maximum: usize,
    random: &mut R,
) -> Result<usize, R::Error> {
    debug_assert!(exclusive_maximum > 0);
    debug_assert!(u32::try_from(exclusive_maximum).is_ok());

    let maximum = exclusive_maximum as u64;
    let range = 1_u64 << 32;
    let ceiling = range - (range % maximum);
    let mut bytes = Zeroizing::new([0_u8; 4]);

    loop {
        random.fill(&mut *bytes)?;
        let value = u32::from_le_bytes(*bytes);
        if u64::from(value) < ceiling {
            return Ok((u64::from(value) % maximum) as usize);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::VecDeque, convert::Infallible};

    use super::*;

    #[derive(Default)]
    struct ZeroSource {
        fills: usize,
    }

    impl RandomSource for ZeroSource {
        type Error = Infallible;

        fn fill(&mut self, destination: &mut [u8]) -> Result<(), Self::Error> {
            assert_eq!(destination.len(), 4);
            destination.fill(0);
            self.fills += 1;
            Ok(())
        }
    }

    struct ScriptedSource {
        values: VecDeque<u32>,
        fills: usize,
    }

    impl ScriptedSource {
        fn new(values: impl IntoIterator<Item = u32>) -> Self {
            Self {
                values: values.into_iter().collect(),
                fills: 0,
            }
        }
    }

    impl RandomSource for ScriptedSource {
        type Error = &'static str;

        fn fill(&mut self, destination: &mut [u8]) -> Result<(), Self::Error> {
            if destination.len() != 4 {
                return Err("unexpected request size");
            }
            let value = self.values.pop_front().ok_or("script exhausted")?;
            destination.copy_from_slice(&value.to_le_bytes());
            self.fills += 1;
            Ok(())
        }
    }

    struct FailingSource;

    impl RandomSource for FailingSource {
        type Error = &'static str;

        fn fill(&mut self, _destination: &mut [u8]) -> Result<(), Self::Error> {
            Err("entropy failed")
        }
    }

    fn options(mask: u8, exclude_ambiguous: bool) -> GenerationOptions {
        GenerationOptions {
            lowercase: mask & 1 != 0,
            uppercase: mask & 2 != 0,
            numbers: mask & 4 != 0,
            symbols: mask & 8 != 0,
            exclude_ambiguous,
        }
    }

    #[test]
    fn exact_groups_and_ambiguous_filter_match_wpf() {
        let unfiltered = build_groups(options(15, false));
        assert_eq!(
            unfiltered,
            vec![
                b"abcdefghijklmnopqrstuvwxyz".to_vec(),
                b"ABCDEFGHIJKLMNOPQRSTUVWXYZ".to_vec(),
                b"0123456789".to_vec(),
                b"!@#$%^&*_-+=?".to_vec(),
            ]
        );

        let filtered = build_groups(options(15, true));
        assert_eq!(
            filtered,
            vec![
                b"abcdefghijkmnpqrstuvwxyz".to_vec(),
                b"ACDEFGHJKLMNPQRTUVWXYZ".to_vec(),
                b"234679".to_vec(),
                b"!@#$%^&*_-+=?".to_vec(),
            ]
        );
    }

    #[test]
    fn all_fifteen_masks_cover_every_selected_group_at_both_length_bounds() {
        for exclude_ambiguous in [false, true] {
            for mask in 1..16 {
                let generation_options = options(mask, exclude_ambiguous);
                let selected_groups = build_groups(generation_options);

                for length in [MIN_LENGTH, MAX_LENGTH] {
                    let mut random = ZeroSource::default();
                    let password =
                        generate_passwords_with_source(1, length, generation_options, &mut random)
                            .unwrap()
                            .pop()
                            .unwrap();

                    assert_eq!(password.len(), length, "mask {mask}");
                    assert!(password.bytes().all(|character| {
                        selected_groups
                            .iter()
                            .any(|group| group.contains(&character))
                    }));
                    for group in &selected_groups {
                        assert!(
                            password.bytes().any(|character| group.contains(&character)),
                            "mask {mask} omitted selected group {}",
                            String::from_utf8_lossy(group)
                        );
                    }
                    if exclude_ambiguous {
                        assert!(
                            !password
                                .bytes()
                                .any(|character| AMBIGUOUS.as_bytes().contains(&character))
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn validates_count_length_and_group_bounds() {
        let enabled = options(15, true);

        for count in [0, MAX_COUNT + 1] {
            let error =
                generate_passwords_with_source(count, 20, enabled, &mut ZeroSource::default())
                    .unwrap_err();
            assert!(
                matches!(error, InternalError::CountOutOfRange { count: value } if value == count)
            );
        }

        for length in [0, MIN_LENGTH - 1, MAX_LENGTH + 1, usize::MAX] {
            let error =
                generate_passwords_with_source(1, length, enabled, &mut ZeroSource::default())
                    .unwrap_err();
            assert!(
                matches!(error, InternalError::LengthOutOfRange { length: value } if value == length)
            );
        }

        let error =
            generate_passwords_with_source(1, 20, options(0, true), &mut ZeroSource::default())
                .unwrap_err();
        assert!(matches!(error, InternalError::NoCharacterGroups));
    }

    #[test]
    fn supports_maximum_batch() {
        let passwords = generate_passwords_with_source(
            MAX_COUNT,
            20,
            options(15, false),
            &mut ZeroSource::default(),
        )
        .unwrap();

        assert_eq!(passwords.len(), MAX_COUNT);
        assert!(passwords.iter().all(|password| password.len() == 20));
    }

    #[test]
    fn rejection_sampling_discards_values_at_or_above_ceiling() {
        let mut random = ScriptedSource::new([u32::MAX, 8]);

        assert_eq!(next_index(10, &mut random).unwrap(), 8);
        assert_eq!(random.fills, 2);
        assert!(random.values.is_empty());
    }

    #[test]
    fn fisher_yates_uses_injected_indices_in_reverse_order() {
        let mut candidate = *b"abcd";
        let mut random = ScriptedSource::new([1, 0, 1]);

        fisher_yates(&mut candidate, &mut random).unwrap();

        assert_eq!(&candidate, b"cdab");
        assert_eq!(random.fills, 3);
        assert!(random.values.is_empty());
    }

    #[test]
    fn entropy_failure_is_returned_without_fallback() {
        let error = generate_passwords_with_source(1, 20, options(15, true), &mut FailingSource)
            .unwrap_err();

        assert_eq!(error, InternalError::Entropy("entropy failed"));
    }
}
