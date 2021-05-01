use rand::distributions::WeightedIndex;
use rand::prelude::*;

use super::{Age, Gender, Generate, Rng};

pub struct Ethnicity;

impl Ethnicity {
    #[rustfmt::skip]
    const FEMININE_NAMES: &'static [&'static str] = &[
        "Aika", "Akemi", "Akiko", "Amaya", "Asami", "Ayumi", "Bunko", "Chieko", "Chika", "Chiyo",
        "Cho", "Eiko", "Emiko", "Eri", "Etsuko", "Gina", "Hana", "Haruki", "Hideko", "Hikari",
        "Hiroko", "Hisoka", "Hishi", "Hotaru", "Izumi", "Kameyo", "Kasumi", "Kimiko", "Kotone",
        "Kyoko", "Maiko", "Masako", "Mi", "Minori", "Mizuki", "Naoki", "Natsuko", "Noriko", "Rei",
        "Ren", "Saki", "Shigeko", "Shinju", "Sumiko", "Toshiko", "Tsukiko", "Ume", "Usagi",
        "Yasuko", "Yuriko",
    ];

    #[rustfmt::skip]
    const MASCULINE_NAMES: &'static [&'static str] = &[
        "Akio", "Atsushi", "Daichi", "Daiki", "Daisuke", "Eiji", "Fumio", "Hajime", "Haru",
        "Hideaki", "Hideo", "Hikaru", "Hiro", "Hiroki", "Hisao", "Hitoshi", "Isamu", "Isao", "Jun",
        "Katashi", "Katsu", "Kei", "Ken", "Kenshin", "Kenta", "Kioshi", "Makoto", "Mamoru",
        "Masato", "Masumi", "Noboru", "Norio", "Osamu", "Ryota", "Sadao", "Satoshi", "Shigeo",
        "Shin", "Sora", "Tadao", "Takehiko", "Takeo", "Takeshi", "Takumi", "Tamotsu", "Tatsuo",
        "Toru", "Toshio", "Yasuo", "Yukio",
    ];
}

impl Generate for Ethnicity {
    fn gen_name(rng: &mut impl Rng, age: &Age, gender: &Gender) -> String {
        match gender {
            Gender::Masculine => {
                Self::MASCULINE_NAMES[rng.gen_range(0..Self::MASCULINE_NAMES.len())].to_string()
            }
            Gender::Feminine => {
                Self::FEMININE_NAMES[rng.gen_range(0..Self::FEMININE_NAMES.len())].to_string()
            }
            _ => {
                let dist =
                    WeightedIndex::new(&[Self::MASCULINE_NAMES.len(), Self::FEMININE_NAMES.len()])
                        .unwrap();
                if dist.sample(rng) == 0 {
                    Self::gen_name(rng, age, &Gender::Masculine)
                } else {
                    Self::gen_name(rng, age, &Gender::Feminine)
                }
            }
        }
    }
}

#[cfg(test)]
mod test_generate_for_ethnicity {
    use super::*;
    use crate::world::npc::ethnicity::{regenerate, Ethnicity};
    use crate::world::Npc;
    use rand::rngs::mock::StepRng;

    #[test]
    fn gen_name_test() {
        let mut rng = StepRng::new(0, 0xDEADBEEF_DECAFBAD);
        let age = Age::Adult(0);
        let m = Gender::Masculine;
        let f = Gender::Feminine;
        let t = Gender::Trans;

        assert_eq!(
            ["Akio", "Takumi", "Maiko", "Haruki", "Daisuke", "Shigeko"],
            [
                gen_name(&mut rng, &age, &m),
                gen_name(&mut rng, &age, &m),
                gen_name(&mut rng, &age, &f),
                gen_name(&mut rng, &age, &f),
                gen_name(&mut rng, &age, &t),
                gen_name(&mut rng, &age, &t),
            ]
        );
    }

    fn gen_name(rng: &mut impl Rng, age: &Age, gender: &Gender) -> String {
        let mut npc = Npc::default();
        npc.gender.replace(*gender);
        npc.age.replace(*age);
        npc.ethnicity.replace(Ethnicity::Japanese);
        regenerate(rng, &mut npc);
        npc.name.value.unwrap()
    }
}
