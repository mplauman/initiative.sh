use rand::distributions::WeightedIndex;
use rand::prelude::*;

use super::{Age, Gender, Generate, Rng};

pub struct Ethnicity;

impl Ethnicity {
    #[rustfmt::skip]
    const FEMININE_NAMES: &'static [&'static str] = &[
        "Akra", "Aasathra", "Antrara", "Arava", "Biri", "Blendaeth", "Burana", "Chassath", "Daar",
        "Dentratha", "Doudra", "Driindar", "Eggren", "Farideh", "Findex", "Furrele", "Gesrethe",
        "Gilkass", "Harann", "Havilar", "Hethress", "Hillanot", "Jaxi", "Jezean", "Jheri",
        "Kadana", "Kava", "Korinn", "Megren", "Mijira", "Mishann", "Nala", "Nuthra", "Perra",
        "Pogranix", "Pyxrin", "Quespa", "Raiann", "Rezena", "Ruloth", "Saphara", "Savaran", "Sora",
        "Surina", "Synthrin", "Tatyan", "Thava", "Uadjit", "Vezera", "Zykroff",
    ];

    #[rustfmt::skip]
    const MASCULINE_NAMES: &'static [&'static str] = &[
        "Adrex", "Arjhan", "Azzakh", "Salasar", "Baradad", "Bharash", "Bidreked", "Dadalan",
        "Dazzazn", "Direcris", "Donaar", "Fax", "Gargax", "Ghesh", "Gorbundus", "Greethen",
        "Heskan", "Hirrathak", "lldrex", "Kaladan", "Kerkad", "Kiirith", "Kriv", "Maagog",
        "Medrash", "Mehen", "Mozikth", "Mreksh", "Mugrunden", "Nadarr", "Nithther", "Norkruuth",
        "Nykkan", "Pandjed", "Patrin", "Pijjirik", "Quarethon", "Rathkran", "Rhogar", "Rivaan",
        "Sethrekar", "Shamash", "Shedinn", "Srorthen", "Tarhun", "Torinn", "Trynnicus", "Valorean",
        "Vrondiss", "Zedaar",
    ];

    #[rustfmt::skip]
    const CLAN_NAMES: &'static [&'static str] = &[
        "Akambherylliax", "Argenthrixus", "Baharoosh", "Beryntolthropal", "Bhenkumbyrznaax",
        "Caavylteradyn", "Chumbyxirinnish", "Clethtinthial", "lor", "Daardendrian", "Delmirev",
        "Dhyrktelonis", "Ebynichtomonis", "Esstyrlynn", "Fharngnarthnost", "Ghaallixirn",
        "Grrrmmballhyst", "Gygazzylyshrift", "Hashphronyxadyn", "Hshhsstoroth", "lmbixtellrhyst",
        "Jerynomonis", "Jharthraxyn", "Kerrhylon", "Kimbatuul", "Lhamboldennish",
        "Linxakasendalor", "Mohradyllion", "Mys", "tan", "Nemmonis", "Norixius", "Ophinshtalajiir",
        "Orexijandilin", "Pfaphnyrennish", "Phrahdrandon", "Pyraxtallinost", "Qyxpahrgh",
        "Raghthroknaar", "Shestendeliath", "Skaarzborroosh", "Sumnarghthrysh", "Tiammanthyllish",
        "Turnuroth", "Umbyrphrael", "Vangdondalor", "Verthisathurgiesh", "Wivvyrholdalphiax",
        "Wystongjiir", "Xephyrbahnor", "Yarjerit", "Zzzxaaxthroth", 
    ];
}

impl Generate for Ethnicity {
    fn gen_name(rng: &mut impl Rng, age: &Age, gender: &Gender) -> String {
        let mut name = Self::CLAN_NAMES[rng.gen_range(0..Self::CLAN_NAMES.len())].to_string();
        name.push(' ');
        name.push_str(match gender {
            Gender::Masculine => {
                Self::MASCULINE_NAMES[rng.gen_range(0..Self::MASCULINE_NAMES.len())]
            }
            Gender::Feminine => Self::FEMININE_NAMES[rng.gen_range(0..Self::FEMININE_NAMES.len())],
            _ => {
                let dist =
                    WeightedIndex::new(&[Self::MASCULINE_NAMES.len(), Self::FEMININE_NAMES.len()])
                        .unwrap();
                if dist.sample(rng) == 0 {
                    return Self::gen_name(rng, age, &Gender::Masculine);
                } else {
                    return Self::gen_name(rng, age, &Gender::Feminine);
                }
            }
        });
        name
    }
}

#[cfg(test)]
mod test_generate_for_ethnicity {
    use super::*;
    use rand::rngs::mock::StepRng;

    #[test]
    fn gen_name_test() {
        let mut rng = StepRng::new(0, 0xDEADBEEF_DECAFBAD);
        let age = Age::Adult(0);
        let m = Gender::Masculine;
        let f = Gender::Feminine;
        let t = Gender::Trans;

        assert_eq!(
            [
                "Akambherylliax Srorthen",
                "Raghthroknaar Nithther",
                "Hashphronyxadyn Biri",
                "Turnuroth Megren",
                "Daardendrian Azzakh",
                "Pfaphnyrennish Kava"
            ],
            [
                Ethnicity::gen_name(&mut rng, &age, &m),
                Ethnicity::gen_name(&mut rng, &age, &m),
                Ethnicity::gen_name(&mut rng, &age, &f),
                Ethnicity::gen_name(&mut rng, &age, &f),
                Ethnicity::gen_name(&mut rng, &age, &t),
                Ethnicity::gen_name(&mut rng, &age, &t),
            ]
        );
    }
}