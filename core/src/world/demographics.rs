use super::npc::{Ethnicity, Species};
use rand::distributions::WeightedIndex;
use rand::prelude::*;
use std::collections::HashMap;
use std::iter;

#[derive(Clone, Debug)]
pub struct Demographics {
    groups: HashMap<(Species, Ethnicity), u64>,
}

impl Demographics {
    pub fn shift_species(&self, species: &Species, amount: f64) -> Self {
        self.shift_by(
            |s, _| s == species,
            amount,
            (*species, species.default_ethnicity()),
        )
    }

    pub fn only_species(&self, species: &Species) -> Self {
        self.shift_species(species, 1.)
    }

    pub fn shift_ethnicity(&self, ethnicity: &Ethnicity, amount: f64) -> Self {
        self.shift_by(
            |_, e| e == ethnicity,
            amount,
            (ethnicity.default_species(), *ethnicity),
        )
    }

    pub fn only_ethnicity(&self, ethnicity: &Ethnicity) -> Self {
        self.shift_ethnicity(ethnicity, 1.)
    }

    pub fn shift_species_ethnicity(
        &self,
        species: &Species,
        ethnicity: &Ethnicity,
        amount: f64,
    ) -> Self {
        self.shift_by(
            |s, e| s == species && e == ethnicity,
            amount,
            (*species, *ethnicity),
        )
    }

    pub fn only_species_ethnicity(&self, species: &Species, ethnicity: &Ethnicity) -> Self {
        self.shift_species_ethnicity(species, ethnicity, 1.)
    }

    pub fn gen_species_ethnicity(&self, rng: &mut impl Rng) -> (Species, Ethnicity) {
        if self.groups.is_empty() {
            (Species::Human, Species::Human.default_ethnicity())
        } else {
            let (groups, weights): (Vec<(Species, Ethnicity)>, Vec<u64>) =
                self.groups.iter().unzip();
            let dist = WeightedIndex::new(&weights).unwrap();
            groups[dist.sample(rng)]
        }
    }

    fn shift_by<F: Fn(&Species, &Ethnicity) -> bool>(
        &self,
        f: F,
        amount: f64,
        default: (Species, Ethnicity),
    ) -> Self {
        if !(0. ..=1.).contains(&amount) {
            panic!("Invalid input: {}", amount);
        }

        let population: u64 = self.groups.values().sum();
        let species_population: u64 = self
            .groups
            .iter()
            .filter_map(|((s, e), n)| if f(s, e) { Some(n) } else { None })
            .sum();

        let groups: HashMap<(Species, Ethnicity), u64> = if species_population > 0 {
            self.groups
                .iter()
                .map(|((s, e), &v)| {
                    (
                        (*s, *e),
                        if f(s, e) {
                            (v as f64 * (1. - amount)
                                + (v as f64 * amount * population as f64
                                    / species_population as f64))
                                .round() as u64
                        } else {
                            (v as f64 * (1. - amount)).round() as u64
                        },
                    )
                })
                .filter(|(_, v)| *v > 0)
                .collect()
        } else {
            self.groups
                .iter()
                .map(|(&k, &v)| (k, (v as f64 * (1. - amount)).round() as u64))
                .chain(iter::once((
                    default,
                    (population as f64 * amount).round() as u64,
                )))
                .filter(|(_, v)| *v > 0)
                .collect()
        };

        Self { groups }
    }
}

#[cfg(test)]
mod test_demographics {
    use super::*;
    use rand::rngs::mock::StepRng;

    #[test]
    fn shift_species_test_existing() {
        let demographics = demographics().shift_species(&Species::Human, 0.3);

        assert_eq!(3, demographics.groups.len());
        assert_eq!(
            Some(&39),
            demographics
                .groups
                .get(&(Species::Human, Ethnicity::Arabic)),
            "{:?}",
            demographics
        );
        assert_eq!(
            Some(&26),
            demographics
                .groups
                .get(&(Species::Human, Ethnicity::Celtic))
        );
        assert_eq!(
            Some(&35),
            demographics
                .groups
                .get(&(Species::Gnome, Ethnicity::Celtic))
        );
    }

    #[test]
    fn shift_ethnicity_test_existing() {
        let demographics = demographics().shift_ethnicity(&Ethnicity::Celtic, 0.3);

        assert_eq!(3, demographics.groups.len());
        assert_eq!(
            Some(&21),
            demographics
                .groups
                .get(&(Species::Human, Ethnicity::Arabic)),
            "{:?}",
            demographics
        );
        assert_eq!(
            Some(&23),
            demographics
                .groups
                .get(&(Species::Human, Ethnicity::Celtic))
        );
        assert_eq!(
            Some(&56),
            demographics
                .groups
                .get(&(Species::Gnome, Ethnicity::Celtic))
        );
    }

    #[test]
    fn shift_species_ethnicity_test_existing() {
        let demographics =
            demographics().shift_species_ethnicity(&Species::Gnome, &Ethnicity::Celtic, 0.3);

        assert_eq!(3, demographics.groups.len());
        assert_eq!(
            Some(&21),
            demographics
                .groups
                .get(&(Species::Human, Ethnicity::Arabic)),
            "{:?}",
            demographics
        );
        assert_eq!(
            Some(&14),
            demographics
                .groups
                .get(&(Species::Human, Ethnicity::Celtic))
        );
        assert_eq!(
            Some(&65),
            demographics
                .groups
                .get(&(Species::Gnome, Ethnicity::Celtic))
        );
    }

    #[test]
    fn shift_species_test_new() {
        let mut groups = HashMap::with_capacity(1);
        groups.insert((Species::Human, Ethnicity::Arabic), 100);
        let demographics = Demographics { groups }.shift_species(&Species::Gnome, 0.4);

        assert_eq!(2, demographics.groups.len());
        assert_eq!(
            Some(&60),
            demographics
                .groups
                .get(&(Species::Human, Ethnicity::Arabic))
        );
        assert_eq!(
            Some(&40),
            demographics
                .groups
                .get(&(Species::Gnome, Ethnicity::Gnomish))
        );
    }

    #[test]
    fn shift_ethnicity_test_new() {
        let mut groups = HashMap::with_capacity(1);
        groups.insert((Species::Human, Ethnicity::Arabic), 100);
        let demographics = Demographics { groups }.shift_ethnicity(&Ethnicity::Gnomish, 0.4);

        assert_eq!(2, demographics.groups.len());
        assert_eq!(
            Some(&60),
            demographics
                .groups
                .get(&(Species::Human, Ethnicity::Arabic))
        );
        assert_eq!(
            Some(&40),
            demographics
                .groups
                .get(&(Species::Gnome, Ethnicity::Gnomish))
        );
    }

    #[test]
    fn shift_species_ethnicity_test_new() {
        let mut groups = HashMap::with_capacity(1);
        groups.insert((Species::Human, Ethnicity::Arabic), 100);
        let demographics = Demographics { groups }.shift_species_ethnicity(
            &Species::Gnome,
            &Ethnicity::Celtic,
            0.4,
        );

        assert_eq!(2, demographics.groups.len());
        assert_eq!(
            Some(&60),
            demographics
                .groups
                .get(&(Species::Human, Ethnicity::Arabic))
        );
        assert_eq!(
            Some(&40),
            demographics
                .groups
                .get(&(Species::Gnome, Ethnicity::Celtic))
        );
    }

    #[test]
    fn only_species_test() {
        let demographics = demographics().only_species(&Species::Human);

        assert_eq!(2, demographics.groups.len());
        assert_eq!(
            Some(&60),
            demographics
                .groups
                .get(&(Species::Human, Ethnicity::Arabic))
        );
        assert_eq!(
            Some(&40),
            demographics
                .groups
                .get(&(Species::Human, Ethnicity::Celtic))
        );
    }

    #[test]
    fn only_ethnicity_test() {
        let demographics = demographics().only_ethnicity(&Ethnicity::Celtic);

        assert_eq!(2, demographics.groups.len());
        assert_eq!(
            Some(&29),
            demographics
                .groups
                .get(&(Species::Human, Ethnicity::Celtic))
        );
        assert_eq!(
            Some(&71),
            demographics
                .groups
                .get(&(Species::Gnome, Ethnicity::Celtic))
        );
    }

    #[test]
    fn only_species_ethnicity_test() {
        let demographics =
            demographics().only_species_ethnicity(&Species::Gnome, &Ethnicity::Celtic);

        assert_eq!(1, demographics.groups.len());
        assert_eq!(
            Some(&100),
            demographics
                .groups
                .get(&(Species::Gnome, Ethnicity::Celtic))
        );
    }

    #[test]
    fn gen_species_ethnicity_test() {
        let mut groups = HashMap::new();
        groups.insert((Species::Human, Ethnicity::Arabic), 50);
        groups.insert((Species::Gnome, Ethnicity::Celtic), 50);
        let demographics = Demographics { groups };

        let mut rng = StepRng::new(0, 0xDEADBEEF_DECAFBAD);
        let mut counts: HashMap<(Species, Ethnicity), u8> = HashMap::with_capacity(2);

        for i in 0..10 {
            let species_ethnicity = &demographics.gen_species_ethnicity(&mut rng);
            *counts.entry(*species_ethnicity).or_default() += 1;
            println!("{}: {:?}", i, counts);
        }

        assert_eq!(Some(&5), counts.get(&(Species::Human, Ethnicity::Arabic)));
        assert_eq!(Some(&5), counts.get(&(Species::Gnome, Ethnicity::Celtic)));
    }

    fn demographics() -> Demographics {
        let mut groups = HashMap::with_capacity(3);
        groups.insert((Species::Human, Ethnicity::Arabic), 30);
        groups.insert((Species::Human, Ethnicity::Celtic), 20);
        groups.insert((Species::Gnome, Ethnicity::Celtic), 50);
        Demographics { groups }
    }
}

impl Default for Demographics {
    fn default() -> Self {
        let mut groups = HashMap::new();
        groups.insert((Species::Human, Ethnicity::Human), 1_020_000);
        groups.insert((Species::HalfElf, Ethnicity::HalfElvish), 320_000);
        groups.insert((Species::Elf, Ethnicity::Elvish), 220_000);
        groups.insert((Species::Gnome, Ethnicity::Gnomish), 220_000);
        groups.insert((Species::Halfling, Ethnicity::Halfling), 100_000);
        // groups.insert(Species::Shifter, 60_000);
        // groups.insert(Species::Changeling, 40_000);

        Self { groups }
    }
}
