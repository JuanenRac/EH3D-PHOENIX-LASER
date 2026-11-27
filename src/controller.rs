// EH3D-PHOENIX-LASER - the settings of the controller, read from a saved listing.
// Copyright (C) 2026 JuanenRac (Electro Hobby 3D). GPL-3.0-or-later.
//! A GRBL controller lists its settings (`$$`) as lines such as `$32=1`. A person saves that listing to a file and this module compares it with what a program assumes: whether the laser mode
//! is on, whether the value of full power is the one the program was written for, the least power, the units, the travel and the speeds. Nothing here opens a port or talks to a machine, and
//! nothing is ever written to a controller. A value that the listing does not give is unknown, and unknown counts as unsafe for the two settings that decide the power of the beam.

use crate::lang::{fill, Lang};
use std::collections::BTreeMap;

/// A listing larger than this is not read.
pub const MAX_DUMP_BYTES: usize = 64 * 1024;

/// The settings of a listing, by their number.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Dump {
    values: BTreeMap<u32, f64>,
}

impl Dump {
    pub fn get(&self, number: u32) -> Option<f64> {
        self.values.get(&number).copied()
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
}

/// Reads a saved listing. Lines such as `$32=1` or `$32=1 (laser mode)` are the settings; every other line (`ok`, the version, the start-up blocks `$N0=`) is left alone. A value that is not a
/// number, or a setting given twice with two values, is an error with the number of its line.
pub fn parse(text: &str) -> Result<Dump, String> {
    if text.len() > MAX_DUMP_BYTES {
        return Err(format!(
            "the listing of the settings is larger than {MAX_DUMP_BYTES} bytes"
        ));
    }
    let mut dump = Dump::default();
    for (n, raw) in text.trim_start_matches('\u{feff}').lines().enumerate() {
        let line = n + 1;
        let Some((key, rest)) = raw.trim().strip_prefix('$').and_then(|t| t.split_once('=')) else {
            continue;
        };
        let Ok(number) = key.trim().parse::<u32>() else {
            continue;
        };
        let value = rest
            .split(|c: char| c.is_whitespace() || c == '(')
            .find(|w| !w.is_empty())
            .unwrap_or("");
        let value = value
            .parse::<f64>()
            .ok()
            .filter(|v| v.is_finite())
            .ok_or_else(|| format!("line {line}: the value of ${number} is not a number"))?;
        if let Some(before) = dump.values.insert(number, value) {
            if before != value {
                return Err(format!(
                    "line {line}: ${number} is given twice, as {before} and as {value}"
                ));
            }
        }
    }
    if dump.is_empty() {
        return Err(
            "no setting such as $32=1 was found: save what the controller answers to $$"
                .to_string(),
        );
    }
    Ok(dump)
}

/// What a program assumes of the controller.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Expect {
    /// The `S` that is full power.
    pub s_max: u32,
    /// The size of the bed (mm), when it is known.
    pub bed_mm: Option<(f64, f64)>,
    /// The highest feed of the program (mm/min), when there is a program.
    pub max_feed_mm_min: Option<f64>,
}

/// What a comparison found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    LaserModeOff,
    LaserModeUnknown,
    SMaxDiffers,
    SMaxUnknown,
    MinPower,
    Inches,
    BedSmaller,
    SpeedLower,
}

impl Kind {
    pub const ALL: [Kind; 8] = [
        Kind::LaserModeOff,
        Kind::LaserModeUnknown,
        Kind::SMaxDiffers,
        Kind::SMaxUnknown,
        Kind::MinPower,
        Kind::Inches,
        Kind::BedSmaller,
        Kind::SpeedLower,
    ];

    /// Whether a job must not be run with it.
    pub fn serious(self) -> bool {
        matches!(
            self,
            Kind::LaserModeOff | Kind::LaserModeUnknown | Kind::SMaxDiffers | Kind::SMaxUnknown
        )
    }

    /// The sentence in each language, in the order of [`Lang::ALL`]; `{0}` and `{1}` are the values.
    pub fn sentences(self) -> [&'static str; 8] {
        match self {
            Kind::LaserModeOff => [
                "The laser mode of the controller is off ($32={0}): the power does not follow the movement and the beam can stay on while the head stands.",
                "El modo láser del controlador está apagado ($32={0}): la potencia no sigue al movimiento y el haz puede quedar encendido con el cabezal parado.",
                "Der Lasermodus der Steuerung ist aus ($32={0}): die Leistung folgt der Bewegung nicht und der Strahl kann bei stehendem Kopf an bleiben.",
                "Le mode laser du contrôleur est désactivé ($32={0}) : la puissance ne suit pas le mouvement et le faisceau peut rester allumé tête arrêtée.",
                "La modalità laser del controllore è spenta ($32={0}): la potenza non segue il movimento e il raggio può restare acceso a testa ferma.",
                "コントローラのレーザーモードがオフです（$32={0}）：出力が移動に追従せず、ヘッドが止まったままビームが点灯し続けることがあります。",
                "控制器的激光模式已关闭（$32={0}）：功率不随运动变化，激光头停止时光束可能仍然开启。",
                "O modo laser do controlador está desligado ($32={0}): a potência não acompanha o movimento e o feixe pode ficar aceso com a cabeça parada.",
            ],
            Kind::LaserModeUnknown => [
                "The listing does not say whether the laser mode is on ($32): what is not known is taken as unsafe.",
                "El listado no dice si el modo láser está activado ($32): lo que no se sabe se considera inseguro.",
                "Die Liste sagt nicht, ob der Lasermodus an ist ($32): Unbekanntes gilt als unsicher.",
                "La liste ne dit pas si le mode laser est activé ($32) : ce qui est inconnu est tenu pour dangereux.",
                "L'elenco non dice se la modalità laser è attiva ($32): ciò che non si sa è considerato non sicuro.",
                "一覧にレーザーモード（$32）がありません：不明なものは安全でないとみなします。",
                "列表没有说明激光模式是否开启（$32）：未知情况按不安全处理。",
                "A listagem não diz se o modo laser está ligado ($32): o que não se sabe é considerado inseguro.",
            ],
            Kind::SMaxDiffers => [
                "Full power of the controller is S{0} ($30) but the program takes S{1} as full power: the power would not be the one asked.",
                "La potencia máxima del controlador es S{0} ($30) pero el programa toma S{1} como potencia máxima: la potencia no sería la pedida.",
                "Volle Leistung der Steuerung ist S{0} ($30), das Programm nimmt aber S{1} als volle Leistung: die Leistung wäre nicht die verlangte.",
                "La pleine puissance du contrôleur est S{0} ($30) mais le programme prend S{1} pour la pleine puissance : la puissance ne serait pas celle demandée.",
                "La piena potenza del controllore è S{0} ($30) ma il programma considera S{1} la piena potenza: la potenza non sarebbe quella richiesta.",
                "コントローラの最大出力は S{0}（$30）ですが、プログラムは S{1} を最大出力としています：指定した出力になりません。",
                "控制器的满功率为 S{0}（$30），而程序把 S{1} 当作满功率：实际功率将与要求不符。",
                "A potência máxima do controlador é S{0} ($30) mas o programa toma S{1} como potência máxima: a potência não seria a pedida.",
            ],
            Kind::SMaxUnknown => [
                "The listing does not give the value of full power ($30): the program takes S{0}, and what is not known is taken as unsafe.",
                "El listado no da el valor de la potencia máxima ($30): el programa toma S{0}, y lo que no se sabe se considera inseguro.",
                "Die Liste nennt den Wert der vollen Leistung nicht ($30): das Programm nimmt S{0}, und Unbekanntes gilt als unsicher.",
                "La liste ne donne pas la valeur de la pleine puissance ($30) : le programme prend S{0}, et l'inconnu est tenu pour dangereux.",
                "L'elenco non dà il valore della piena potenza ($30): il programma usa S{0}, e ciò che non si sa è considerato non sicuro.",
                "一覧に最大出力の値（$30）がありません：プログラムは S{0} としていますが、不明なものは安全でないとみなします。",
                "列表没有给出满功率值（$30）：程序按 S{0} 处理，而未知情况按不安全处理。",
                "A listagem não indica o valor da potência máxima ($30): o programa toma S{0}, e o que não se sabe é considerado inseguro.",
            ],
            Kind::MinPower => [
                "The least value of S is {0} ($31): a power of zero in the program may not be zero.",
                "El valor mínimo de S es {0} ($31): una potencia cero del programa puede no ser cero.",
                "Der kleinste S-Wert ist {0} ($31): eine Leistung null im Programm ist vielleicht nicht null.",
                "La plus petite valeur de S est {0} ($31) : une puissance nulle du programme peut ne pas être nulle.",
                "Il valore minimo di S è {0} ($31): una potenza zero del programma potrebbe non essere zero.",
                "S の最小値は {0} です（$31）：プログラムの出力ゼロがゼロにならないことがあります。",
                "S 的最小值为 {0}（$31）：程序中的零功率可能并非零。",
                "O valor mínimo de S é {0} ($31): uma potência zero do programa pode não ser zero.",
            ],
            Kind::Inches => [
                "The controller reports in inches ($13=1): the positions it shows are not millimetres.",
                "El controlador informa en pulgadas ($13=1): las posiciones que muestra no son milímetros.",
                "Die Steuerung meldet in Zoll ($13=1): die gezeigten Positionen sind keine Millimeter.",
                "Le contrôleur rapporte en pouces ($13=1) : les positions affichées ne sont pas en millimètres.",
                "Il controllore riporta in pollici ($13=1): le posizioni mostrate non sono millimetri.",
                "コントローラはインチで報告します（$13=1）：表示される位置はミリメートルではありません。",
                "控制器以英寸报告（$13=1）：显示的位置不是毫米。",
                "O controlador informa em polegadas ($13=1): as posições mostradas não são milímetros.",
            ],
            Kind::BedSmaller => [
                "The travel of the controller, {0} mm, is smaller than the bed of the settings, {1} mm.",
                "El recorrido del controlador, {0} mm, es menor que la cama de los ajustes, {1} mm.",
                "Der Verfahrweg der Steuerung, {0} mm, ist kleiner als das Bett der Einstellungen, {1} mm.",
                "La course du contrôleur, {0} mm, est plus petite que le plateau des réglages, {1} mm.",
                "La corsa del controllore, {0} mm, è più piccola del piano delle impostazioni, {1} mm.",
                "コントローラの移動範囲 {0} mm は、設定のベッド {1} mm より小さいです。",
                "控制器的行程 {0} mm 小于设置中的工作台尺寸 {1} mm。",
                "O curso do controlador, {0} mm, é menor do que a mesa das definições, {1} mm.",
            ],
            Kind::SpeedLower => [
                "The fastest the controller moves is {0} mm/min and the program asks for {1} mm/min: it would move slower and put more energy into the material.",
                "Lo más rápido que mueve el controlador es {0} mm/min y el programa pide {1} mm/min: iría más despacio y metería más energía en el material.",
                "Die Steuerung fährt höchstens {0} mm/min, das Programm verlangt {1} mm/min: es führe langsamer und brächte mehr Energie ins Material.",
                "Le contrôleur va au plus à {0} mm/min et le programme demande {1} mm/min : il irait plus lentement et mettrait plus d'énergie dans la matière.",
                "Il controllore va al massimo a {0} mm/min e il programma chiede {1} mm/min: andrebbe più piano e metterebbe più energia nel materiale.",
                "コントローラの最高速度は {0} mm/分ですが、プログラムは {1} mm/分を要求しています：遅く動き、材料に入るエネルギーが増えます。",
                "控制器最快为 {0} mm/min，而程序要求 {1} mm/min：实际会更慢，进入材料的能量更多。",
                "O mais rápido que o controlador se move é {0} mm/min e o programa pede {1} mm/min: iria mais devagar e poria mais energia no material.",
            ],
        }
    }
}

/// One finding with the values of its sentence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    pub kind: Kind,
    pub values: Vec<String>,
}

impl Finding {
    pub fn text(&self, lang: Lang) -> String {
        let i = Lang::ALL.iter().position(|l| *l == lang).unwrap_or(0);
        let values: Vec<&str> = self.values.iter().map(String::as_str).collect();
        fill(self.kind.sentences()[i], &values)
    }
}

/// Compares a listing with what a program assumes; the serious findings come first.
pub fn check(dump: &Dump, expect: &Expect) -> Vec<Finding> {
    let mut found: Vec<Finding> = Vec::new();
    let mut add = |kind: Kind, values: Vec<String>| found.push(Finding { kind, values });
    match dump.get(32) {
        Some(v) if v == 1.0 => {}
        Some(v) => add(Kind::LaserModeOff, vec![v.to_string()]),
        None => add(Kind::LaserModeUnknown, Vec::new()),
    }
    match dump.get(30) {
        Some(v) if (v - f64::from(expect.s_max)).abs() < 0.5 => {}
        Some(v) => add(
            Kind::SMaxDiffers,
            vec![v.to_string(), expect.s_max.to_string()],
        ),
        None => add(Kind::SMaxUnknown, vec![expect.s_max.to_string()]),
    }
    if let Some(v) = dump.get(31).filter(|v| *v > 0.0) {
        add(Kind::MinPower, vec![v.to_string()]);
    }
    if dump.get(13) == Some(1.0) {
        add(Kind::Inches, Vec::new());
    }
    if let Some((bx, by)) = expect.bed_mm {
        for (number, bed) in [(130, bx), (131, by)] {
            if let Some(travel) = dump.get(number).filter(|t| *t + 1e-6 < bed) {
                add(Kind::BedSmaller, vec![travel.to_string(), bed.to_string()]);
            }
        }
    }
    if let Some(feed) = expect.max_feed_mm_min {
        let slowest = [dump.get(110), dump.get(111)]
            .into_iter()
            .flatten()
            .fold(f64::INFINITY, f64::min);
        if feed > slowest + 1e-6 {
            add(
                Kind::SpeedLower,
                vec![slowest.to_string(), feed.to_string()],
            );
        }
    }
    found
}
