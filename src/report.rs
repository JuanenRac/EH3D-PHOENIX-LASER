// EH3D-PHOENIX-LASER - the report that is read before a job is started.
// Copyright (C) 2026 JuanenRac (Electro Hobby 3D). GPL-3.0-or-later.
//! Before a laser program is run, the person who starts it should be able to read, in their own language, what it will do and what was checked: which file (its SHA-256), how long and
//! how far it burns, where, at what power and speed against the limits, the energy when the power of the laser is known, whether the material was verified, how many forbidden zones and what
//! the interlocks say, and the verdict. The words of the report are in the eight languages of the program (English, Spanish, German, French, Italian, Japanese, Chinese and Portuguese of
//! Portugal); the reasons that the check gives (`problems` and `notes`) are written in English by the check itself and are shown as they are.

use crate::guard::{AirAssist, Enclosure, Estop, Interlocks};
use crate::lang::{fill, Lang};
use crate::preview::{clock, Report};
use crate::verify::{Limits, Verdict};

/// Every fixed text of the report.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    Title,
    Program,
    Hash,
    Moves,
    WorkBox,
    NoWorkBox,
    Power,
    Speed,
    Energy,
    MaterialVerified,
    MaterialNotVerified,
    MaterialUnmarked,
    NoMaterial,
    Zones,
    NoZones,
    Interlocks,
    InterlocksOff,
    EstopReleased,
    EstopPressed,
    EnclosureClosed,
    EnclosureOpen,
    EnclosureUnknown,
    AirOn,
    AirOff,
    AirUnknown,
    Problems,
    Notes,
    Pass,
    Fail,
    Reminder,
}

impl Key {
    /// Every key, to check that none is missing a language.
    pub const ALL: [Key; 30] = [
        Key::Title,
        Key::Program,
        Key::Hash,
        Key::Moves,
        Key::WorkBox,
        Key::NoWorkBox,
        Key::Power,
        Key::Speed,
        Key::Energy,
        Key::MaterialVerified,
        Key::MaterialNotVerified,
        Key::MaterialUnmarked,
        Key::NoMaterial,
        Key::Zones,
        Key::NoZones,
        Key::Interlocks,
        Key::InterlocksOff,
        Key::EstopReleased,
        Key::EstopPressed,
        Key::EnclosureClosed,
        Key::EnclosureOpen,
        Key::EnclosureUnknown,
        Key::AirOn,
        Key::AirOff,
        Key::AirUnknown,
        Key::Problems,
        Key::Notes,
        Key::Pass,
        Key::Fail,
        Key::Reminder,
    ];

    /// The text in a language, in the order of [`Lang::ALL`]. `{0}`, `{1}`... are the places of the values.
    fn texts(self) -> [&'static str; 8] {
        match self {
            Key::Title => [
                "Report before the job",
                "Informe previo del trabajo",
                "Bericht vor dem Auftrag",
                "Rapport avant le travail",
                "Rapporto prima del lavoro",
                "ジョブ開始前のレポート",
                "作业前报告",
                "Relatório prévio do trabalho",
            ],
            Key::Program => [
                "Program: {0}",
                "Programa: {0}",
                "Programm: {0}",
                "Programme : {0}",
                "Programma: {0}",
                "プログラム：{0}",
                "程序：{0}",
                "Programa: {0}",
            ],
            Key::Hash => [
                "SHA-256 of the file: {0}",
                "SHA-256 del fichero: {0}",
                "SHA-256 der Datei: {0}",
                "SHA-256 du fichier : {0}",
                "SHA-256 del file: {0}",
                "ファイルの SHA-256：{0}",
                "文件的 SHA-256：{0}",
                "SHA-256 do ficheiro: {0}",
            ],
            Key::Moves => [
                "{0} moves, {1} mm burnt, {2} mm of travel, about {3}",
                "{0} movimientos, {1} mm quemados, {2} mm de desplazamiento, unos {3}",
                "{0} Bewegungen, {1} mm gebrannt, {2} mm Leerfahrt, etwa {3}",
                "{0} déplacements, {1} mm brûlés, {2} mm à vide, environ {3}",
                "{0} movimenti, {1} mm bruciati, {2} mm di spostamento, circa {3}",
                "{0} 回の移動、照射 {1} mm、空走 {2} mm、約 {3}",
                "{0} 次移动，烧蚀 {1} 毫米，空行程 {2} 毫米，约 {3}",
                "{0} movimentos, {1} mm queimados, {2} mm de deslocação, cerca de {3}",
            ],
            Key::WorkBox => [
                "Work box: {0} x {1} mm, from {2},{3}",
                "Caja del trabajo: {0} x {1} mm, desde {2},{3}",
                "Arbeitsbereich: {0} x {1} mm, ab {2},{3}",
                "Zone de travail : {0} x {1} mm, depuis {2},{3}",
                "Area di lavoro: {0} x {1} mm, da {2},{3}",
                "加工範囲：{0} x {1} mm（起点 {2},{3}）",
                "加工范围：{0} x {1} 毫米，起点 {2},{3}",
                "Caixa do trabalho: {0} x {1} mm, a partir de {2},{3}",
            ],
            Key::NoWorkBox => [
                "Work box: nothing moves",
                "Caja del trabajo: no se mueve nada",
                "Arbeitsbereich: nichts bewegt sich",
                "Zone de travail : rien ne bouge",
                "Area di lavoro: non si muove nulla",
                "加工範囲：動きはありません",
                "加工范围：没有任何移动",
                "Caixa do trabalho: nada se move",
            ],
            Key::Power => [
                "Highest power: S{0} ({1} % of full power); the limit is {2} %",
                "Potencia máxima: S{0} ({1} % de la potencia total); el límite es {2} %",
                "Höchste Leistung: S{0} ({1} % der vollen Leistung); die Grenze ist {2} %",
                "Puissance maximale : S{0} ({1} % de la pleine puissance) ; la limite est {2} %",
                "Potenza massima: S{0} ({1} % della potenza piena); il limite è {2} %",
                "最大出力：S{0}（全出力の {1} %）、上限は {2} %",
                "最大功率：S{0}（满功率的 {1} %），上限为 {2} %",
                "Potência máxima: S{0} ({1} % da potência total); o limite é {2} %",
            ],
            Key::Speed => [
                "Highest speed: {0} mm/min; the limit is {1} mm/min",
                "Velocidad máxima: {0} mm/min; el límite es {1} mm/min",
                "Höchste Geschwindigkeit: {0} mm/min; die Grenze ist {1} mm/min",
                "Vitesse maximale : {0} mm/min ; la limite est {1} mm/min",
                "Velocità massima: {0} mm/min; il limite è {1} mm/min",
                "最大速度：{0} mm/min、上限は {1} mm/min",
                "最大速度：{0} 毫米/分钟，上限为 {1} 毫米/分钟",
                "Velocidade máxima: {0} mm/min; o limite é {1} mm/min",
            ],
            Key::Energy => [
                "Energy: {0} J from a laser of {1} W ({2} W on average while it burns)",
                "Energía: {0} J de un láser de {1} W ({2} W de media mientras quema)",
                "Energie: {0} J von einem Laser mit {1} W (im Mittel {2} W beim Brennen)",
                "Énergie : {0} J d'un laser de {1} W ({2} W en moyenne pendant qu'il brûle)",
                "Energia: {0} J da un laser da {1} W ({2} W in media mentre brucia)",
                "エネルギー：{1} W レーザーで {0} J（照射中の平均 {2} W）",
                "能量：{1} 瓦激光器共 {0} 焦耳（烧蚀时平均 {2} 瓦）",
                "Energia: {0} J de um laser de {1} W ({2} W em média enquanto queima)",
            ],
            Key::MaterialVerified => [
                "Material {0}: verified",
                "Material {0}: verificado",
                "Material {0}: geprüft",
                "Matériau {0} : vérifié",
                "Materiale {0}: verificato",
                "材料 {0}：検証済み",
                "材料 {0}：已验证",
                "Material {0}: verificado",
            ],
            Key::MaterialNotVerified => [
                "Material {0}: NOT VERIFIED, its values are only a place to start from",
                "Material {0}: SIN VERIFICAR, sus valores son solo un punto de partida",
                "Material {0}: NICHT GEPRÜFT, die Werte sind nur ein Ausgangspunkt",
                "Matériau {0} : NON VÉRIFIÉ, ses valeurs ne sont qu'un point de départ",
                "Materiale {0}: NON VERIFICATO, i suoi valori sono solo un punto di partenza",
                "材料 {0}：未検証。値は出発点にすぎません",
                "材料 {0}：未验证，其数值只是一个起点",
                "Material {0}: NÃO VERIFICADO, os seus valores são apenas um ponto de partida",
            ],
            Key::MaterialUnmarked => [
                "Material {0}: nobody has said whether it was verified",
                "Material {0}: nadie ha dicho si está verificado",
                "Material {0}: niemand hat angegeben, ob es geprüft wurde",
                "Matériau {0} : personne n'a dit s'il a été vérifié",
                "Materiale {0}: nessuno ha detto se è stato verificato",
                "材料 {0}：検証済みかどうかは記載されていません",
                "材料 {0}：没有人说明它是否已验证",
                "Material {0}: ninguém disse se foi verificado",
            ],
            Key::NoMaterial => [
                "Material: none was chosen",
                "Material: no se eligió ninguno",
                "Material: keines gewählt",
                "Matériau : aucun n'a été choisi",
                "Materiale: nessuno scelto",
                "材料：選択されていません",
                "材料：未选择",
                "Material: nenhum foi escolhido",
            ],
            Key::Zones => [
                "Forbidden zones checked: {0}",
                "Zonas prohibidas comprobadas: {0}",
                "Geprüfte Sperrzonen: {0}",
                "Zones interdites vérifiées : {0}",
                "Zone vietate controllate: {0}",
                "確認した禁止ゾーン：{0}",
                "已检查的禁区：{0}",
                "Zonas proibidas verificadas: {0}",
            ],
            Key::NoZones => [
                "Forbidden zones: none are set",
                "Zonas prohibidas: no hay ninguna definida",
                "Sperrzonen: keine festgelegt",
                "Zones interdites : aucune n'est définie",
                "Zone vietate: nessuna definita",
                "禁止ゾーン：設定されていません",
                "禁区：未设置",
                "Zonas proibidas: não há nenhuma definida",
            ],
            Key::Interlocks => [
                "Interlocks (as the settings file says them): emergency stop {0}, enclosure {1}, air assist {2}",
                "Enclavamientos (según el fichero de ajustes): parada de emergencia {0}, cerramiento {1}, asistencia de aire {2}",
                "Verriegelungen (laut Einstellungsdatei): Not-Aus {0}, Gehäuse {1}, Luftunterstützung {2}",
                "Verrouillages (selon le fichier de réglages) : arrêt d'urgence {0}, capot {1}, assistance d'air {2}",
                "Interblocchi (come dice il file delle impostazioni): arresto di emergenza {0}, cabina {1}, aria assistita {2}",
                "インターロック（設定ファイルの記載）：非常停止 {0}、筐体 {1}、エアアシスト {2}",
                "联锁（按设置文件所述）：急停 {0}，机罩 {1}，辅助气流 {2}",
                "Bloqueios (segundo o ficheiro de definições): paragem de emergência {0}, cobertura {1}, assistência de ar {2}",
            ],
            Key::InterlocksOff => [
                "Interlocks: nothing is required and the emergency stop is released",
                "Enclavamientos: no se exige nada y la parada de emergencia está suelta",
                "Verriegelungen: nichts verlangt, der Not-Aus ist gelöst",
                "Verrouillages : rien n'est exigé et l'arrêt d'urgence est relâché",
                "Interblocchi: non si richiede nulla e l'arresto di emergenza è rilasciato",
                "インターロック：要求はなく、非常停止は解除されています",
                "联锁：没有任何要求，急停已松开",
                "Bloqueios: não se exige nada e a paragem de emergência está solta",
            ],
            Key::EstopReleased => [
                "released",
                "suelta",
                "gelöst",
                "relâché",
                "rilasciato",
                "解除",
                "已松开",
                "solta",
            ],
            Key::EstopPressed => [
                "PRESSED",
                "PULSADA",
                "GEDRÜCKT",
                "ENFONCÉ",
                "PREMUTO",
                "押下中",
                "已按下",
                "PREMIDA",
            ],
            Key::EnclosureClosed => [
                "closed",
                "cerrado",
                "geschlossen",
                "fermé",
                "chiusa",
                "閉",
                "已关闭",
                "fechada",
            ],
            Key::EnclosureOpen => [
                "OPEN",
                "ABIERTO",
                "OFFEN",
                "OUVERT",
                "APERTA",
                "開",
                "已打开",
                "ABERTA",
            ],
            Key::EnclosureUnknown => [
                "not known",
                "desconocido",
                "unbekannt",
                "inconnu",
                "sconosciuto",
                "不明",
                "未知",
                "desconhecida",
            ],
            Key::AirOn => ["on", "activada", "ein", "activée", "attiva", "オン", "已开启", "ligada"],
            Key::AirOff => [
                "OFF",
                "DESACTIVADA",
                "AUS",
                "DÉSACTIVÉE",
                "SPENTA",
                "オフ",
                "已关闭",
                "DESLIGADA",
            ],
            Key::AirUnknown => [
                "not known",
                "desconocida",
                "unbekannt",
                "inconnue",
                "sconosciuta",
                "不明",
                "未知",
                "desconhecida",
            ],
            Key::Problems => [
                "Problems (the reasons are given in English):",
                "Problemas (los motivos se dan en inglés):",
                "Probleme (die Gründe stehen auf Englisch):",
                "Problèmes (les raisons sont données en anglais) :",
                "Problemi (i motivi sono in inglese):",
                "問題（理由は英語で示されます）：",
                "问题（原因以英文给出）：",
                "Problemas (os motivos são dados em inglês):",
            ],
            Key::Notes => [
                "Notes (in English):",
                "Notas (en inglés):",
                "Hinweise (auf Englisch):",
                "Remarques (en anglais) :",
                "Note (in inglese):",
                "注記（英語）：",
                "备注（英文）：",
                "Notas (em inglês):",
            ],
            Key::Pass => [
                "VERDICT: the program passes the check",
                "VEREDICTO: el programa pasa la comprobación",
                "ERGEBNIS: das Programm hat die Prüfung bestanden",
                "VERDICT : le programme réussit la vérification",
                "ESITO: il programma supera il controllo",
                "判定：プログラムは検査に合格しました",
                "结论：程序通过检查",
                "VEREDICTO: o programa passa a verificação",
            ],
            Key::Fail => [
                "VERDICT: the program FAILS the check and must not be run",
                "VEREDICTO: el programa NO pasa la comprobación y no debe ejecutarse",
                "ERGEBNIS: das Programm hat die Prüfung NICHT bestanden und darf nicht laufen",
                "VERDICT : le programme ÉCHOUE à la vérification et ne doit pas être lancé",
                "ESITO: il programma NON supera il controllo e non va eseguito",
                "判定：プログラムは検査に不合格です。実行してはいけません",
                "结论：程序未通过检查，不得运行",
                "VEREDICTO: o programa NÃO passa a verificação e não deve ser executado",
            ],
            Key::Reminder => [
                "This report comes from reading the file; nothing was run on a machine. Look at the material, the focus, the enclosure and the beam yourself before you start.",
                "Este informe sale de leer el fichero; no se ha ejecutado nada en una máquina. Mire usted mismo el material, el foco, el cerramiento y el haz antes de empezar.",
                "Dieser Bericht stammt aus dem Lesen der Datei; auf einer Maschine wurde nichts ausgeführt. Prüfen Sie vor dem Start selbst Material, Fokus, Gehäuse und Strahl.",
                "Ce rapport vient de la lecture du fichier ; rien n'a été exécuté sur une machine. Regardez vous-même le matériau, la mise au point, le capot et le faisceau avant de commencer.",
                "Questo rapporto nasce dalla lettura del file; nulla è stato eseguito su una macchina. Controlli lei stesso il materiale, il fuoco, la cabina e il raggio prima di iniziare.",
                "このレポートはファイルを読んだ結果です。機械では何も実行していません。開始前に、材料、焦点、筐体、ビームをご自身で確認してください。",
                "本报告来自对文件的读取，没有在机器上运行任何东西。开始前请亲自查看材料、焦点、机罩和光束。",
                "Este relatório resulta de ler o ficheiro; nada foi executado numa máquina. Veja o senhor mesmo o material, o foco, a cobertura e o feixe antes de começar.",
            ],
        }
    }

    /// The text of the key in a language.
    pub fn text(self, lang: Lang) -> &'static str {
        self.texts()[lang as usize]
    }
}

/// What the report is made from.
pub struct Input<'a> {
    /// The name of the program file as it is shown.
    pub program: &'a str,
    pub verdict: &'a Verdict,
    pub limits: &'a Limits,
    /// The analysis of the program, with the energy when the power of the laser was given.
    pub analysis: &'a Report,
    /// The power of the laser at full power (W), when it is known.
    pub laser_watts: Option<f64>,
    /// The material that was chosen: its name and whether it was verified (`None` when the library does not say).
    pub material: Option<(&'a str, Option<bool>)>,
    /// How many forbidden zones were checked.
    pub zones: usize,
    /// The interlocks of the settings.
    pub interlocks: &'a Interlocks,
}

/// A number as short text (no more than 3 decimals).
fn short(v: f64) -> String {
    let t = format!("{v:.3}");
    t.trim_end_matches('0').trim_end_matches('.').to_string()
}

/// The report as lines of text in a language.
pub fn render(i: &Input, lang: Lang) -> String {
    let t = |k: Key| k.text(lang);
    let v = i.verdict;
    let mut out = vec![
        format!("== {} ==", t(Key::Title)),
        fill(t(Key::Program), &[i.program]),
        fill(t(Key::Hash), &[&v.sha256]),
        fill(
            t(Key::Moves),
            &[
                &v.moves.to_string(),
                &format!("{:.0}", v.burn_mm),
                &format!("{:.0}", v.travel_mm),
                &clock(v.seconds),
            ],
        ),
    ];
    out.push(match v.bounds {
        Some((lo, hi)) => fill(
            t(Key::WorkBox),
            &[
                &short(hi.0 - lo.0),
                &short(hi.1 - lo.1),
                &short(lo.0),
                &short(lo.1),
            ],
        ),
        None => t(Key::NoWorkBox).to_string(),
    });
    out.push(fill(
        t(Key::Power),
        &[
            &short(v.max_s),
            &format!("{:.1}", v.max_power_percent),
            &short(i.limits.max_power_percent),
        ],
    ));
    out.push(fill(
        t(Key::Speed),
        &[&short(v.max_feed), &short(i.limits.max_speed_mm_min)],
    ));
    if let (Some(e), Some(w), Some(mean)) =
        (i.analysis.energy_j, i.laser_watts, i.analysis.mean_power_w)
    {
        out.push(fill(
            t(Key::Energy),
            &[&format!("{e:.1}"), &short(w), &format!("{mean:.2}")],
        ));
    }
    out.push(match i.material {
        Some((name, Some(true))) => fill(t(Key::MaterialVerified), &[name]),
        Some((name, Some(false))) => fill(t(Key::MaterialNotVerified), &[name]),
        Some((name, None)) => fill(t(Key::MaterialUnmarked), &[name]),
        None => t(Key::NoMaterial).to_string(),
    });
    out.push(if i.zones == 0 {
        t(Key::NoZones).to_string()
    } else {
        fill(t(Key::Zones), &[&i.zones.to_string()])
    });
    out.push(
        if i.interlocks.is_active()
            || i.interlocks.enclosure != Enclosure::Unknown
            || i.interlocks.air_assist != AirAssist::Unknown
        {
            let estop = match i.interlocks.estop {
                Estop::Released => Key::EstopReleased,
                Estop::Pressed => Key::EstopPressed,
            };
            let enclosure = match i.interlocks.enclosure {
                Enclosure::Closed => Key::EnclosureClosed,
                Enclosure::Open => Key::EnclosureOpen,
                Enclosure::Unknown => Key::EnclosureUnknown,
            };
            let air = match i.interlocks.air_assist {
                AirAssist::On => Key::AirOn,
                AirAssist::Off => Key::AirOff,
                AirAssist::Unknown => Key::AirUnknown,
            };
            fill(t(Key::Interlocks), &[t(estop), t(enclosure), t(air)])
        } else {
            t(Key::InterlocksOff).to_string()
        },
    );
    if !v.problems.is_empty() {
        out.push(t(Key::Problems).to_string());
        out.extend(v.problems.iter().map(|p| format!("  - {p}")));
    }
    if !v.notes.is_empty() {
        out.push(t(Key::Notes).to_string());
        out.extend(v.notes.iter().map(|n| format!("  - {n}")));
    }
    out.push(
        if v.passes() {
            t(Key::Pass)
        } else {
            t(Key::Fail)
        }
        .to_string(),
    );
    out.push(t(Key::Reminder).to_string());
    let mut text = out.join("\n");
    text.push('\n');
    text
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::guard::{Enclosure, Interlocks};
    use crate::preview::{analyse, read, Options};
    use crate::verify::verify;

    const PROGRAM: &str =
        "G21\nG90\nM4 S0\nF600\nG0 X5 Y5\nG1 X45 Y5 S250\nG1 X45 Y25 S250\nS0\nM5\nM2\n";

    fn input_of<'a>(v: &'a Verdict, l: &'a Limits, a: &'a Report, i: &'a Interlocks) -> Input<'a> {
        Input {
            program: "p.nc",
            verdict: v,
            limits: l,
            analysis: a,
            laser_watts: Some(10.0),
            material: Some(("birch", Some(false))),
            zones: 2,
            interlocks: i,
        }
    }

    fn parts(bytes: &[u8]) -> (Verdict, Limits, Report) {
        let l = Limits::default();
        let v = verify(bytes, &l).unwrap();
        let a = analyse(
            &read(&String::from_utf8_lossy(bytes)),
            &Options {
                s_max: Some(1000),
                laser_watts: Some(10.0),
                ..Options::default()
            },
        )
        .unwrap();
        (v, l, a)
    }

    /// The places `{n}` that a text holds, in order.
    fn places(text: &str) -> Vec<String> {
        let mut out = Vec::new();
        let mut rest = text;
        while let Some(at) = rest.find('{') {
            let Some(end) = rest[at..].find('}') else {
                break;
            };
            out.push(rest[at..at + end + 1].to_string());
            rest = &rest[at + end + 1..];
        }
        out.sort();
        out
    }

    #[test]
    fn every_text_of_the_report_is_there_in_the_eight_languages() {
        for key in Key::ALL {
            let en = key.text(Lang::En);
            assert!(!en.is_empty(), "{key:?}");
            for lang in Lang::ALL {
                let text = key.text(lang);
                assert!(!text.trim().is_empty(), "{key:?} {lang:?}");
                // the same places for the values in every language
                assert_eq!(places(text), places(en), "{key:?} {lang:?}");
                assert_eq!(text, text.trim(), "{key:?} {lang:?} has space at its ends");
            }
        }
        // the list holds every key once
        let mut all = Key::ALL.to_vec();
        all.dedup();
        assert_eq!(all.len(), 30);
    }

    #[test]
    fn the_languages_are_not_copies_of_english_and_portugal_is_not_spanish() {
        for key in Key::ALL {
            for lang in [Lang::Es, Lang::De, Lang::Ja, Lang::Zh] {
                let words_only = key.text(Lang::En).chars().all(|c| !c.is_alphabetic());
                if !words_only {
                    assert_ne!(key.text(lang), key.text(Lang::En), "{key:?} {lang:?}");
                }
            }
        }
        // Portuguese of Portugal: the words of the screen and the file are the ones of Portugal
        let all_pt: String = Key::ALL
            .iter()
            .map(|k| k.text(Lang::Pt))
            .collect::<Vec<_>>()
            .join(" ");
        assert!(all_pt.contains("ficheiro"), "{all_pt}");
        for brazilian in ["arquivo", "tela", "celular", "você"] {
            assert!(!all_pt.contains(brazilian), "{brazilian}");
        }
        assert_ne!(Key::Hash.text(Lang::Pt), Key::Hash.text(Lang::Es));
    }

    #[test]
    fn the_report_of_a_good_program_gives_its_hash_its_numbers_and_a_pass_in_every_language() {
        let (v, l, a) = parts(PROGRAM.as_bytes());
        let none = Interlocks::default();
        for lang in Lang::ALL {
            let text = render(&input_of(&v, &l, &a, &none), lang);
            assert!(text.contains(&v.sha256), "{lang:?}");
            assert!(text.contains("p.nc"), "{lang:?}");
            assert!(
                text.contains("S250") && text.contains("25.0"),
                "{lang:?}: {text}"
            );
            assert!(text.contains("45 x 25"), "{lang:?}: {text}");
            assert!(text.contains(Key::Pass.text(lang)), "{lang:?}");
            assert!(!text.contains(Key::Fail.text(lang)), "{lang:?}");
            assert!(
                !text.contains("{0}") && !text.contains("{1}") && !text.contains("{2}"),
                "{lang:?}: {text}"
            );
            assert!(text.contains(Key::Reminder.text(lang)));
        }
    }

    #[test]
    fn the_report_says_what_the_material_the_zones_and_the_interlocks_are() {
        let (v, l, a) = parts(PROGRAM.as_bytes());
        let none = Interlocks::default();
        let mut input = input_of(&v, &l, &a, &none);
        let es = render(&input, Lang::Es);
        assert!(es.contains("SIN VERIFICAR") && es.contains("birch"), "{es}");
        assert!(es.contains("Zonas prohibidas comprobadas: 2"), "{es}");
        assert!(es.contains(Key::InterlocksOff.text(Lang::Es)), "{es}");
        assert!(es.contains("J de un láser de 10 W"), "{es}");
        input.material = Some(("birch", Some(true)));
        assert!(render(&input, Lang::De).contains("geprüft"));
        input.material = Some(("birch", None));
        assert!(render(&input, Lang::Fr).contains("personne n'a dit"));
        input.material = None;
        assert!(render(&input, Lang::It).contains("nessuno scelto"));
        input.zones = 0;
        assert!(render(&input, Lang::Pt).contains("não há nenhuma definida"));
        input.laser_watts = None;
        assert!(!render(&input, Lang::En).contains("Energy:"));
        // interlocks that were set are told word by word
        let set = Interlocks {
            estop: Estop::Pressed,
            enclosure: Enclosure::Open,
            air_assist: AirAssist::Off,
            require_enclosure: true,
            require_air_assist: false,
        };
        let input = input_of(&v, &l, &a, &set);
        let en = render(&input, Lang::En);
        assert!(
            en.contains("emergency stop PRESSED, enclosure OPEN, air assist OFF"),
            "{en}"
        );
        let ja = render(&input, Lang::Ja);
        assert!(
            ja.contains("非常停止 押下中、筐体 開、エアアシスト オフ"),
            "{ja}"
        );
    }

    #[test]
    fn a_program_that_fails_is_reported_with_its_reasons_and_a_verdict_that_says_not_to_run_it() {
        let hot = "G21\nG90\nM4 S0\nF600\nG0 X5 Y5\nG1 X45 Y5 S900\nS0\nM5\nM2\n";
        let l = Limits {
            max_power_percent: 50.0,
            ..Limits::default()
        };
        let v = verify(hot.as_bytes(), &l).unwrap();
        let a = analyse(&read(hot), &Options::default()).unwrap();
        let none = Interlocks::default();
        let input = Input {
            program: "hot.nc",
            verdict: &v,
            limits: &l,
            analysis: &a,
            laser_watts: None,
            material: None,
            zones: 0,
            interlocks: &none,
        };
        for lang in Lang::ALL {
            let text = render(&input, lang);
            assert!(text.contains(Key::Fail.text(lang)), "{lang:?}");
            assert!(text.contains(Key::Problems.text(lang)), "{lang:?}");
            assert!(
                text.contains("more than the limit of power"),
                "{lang:?}: {text}"
            );
            assert!(!text.contains(Key::Pass.text(lang)), "{lang:?}");
        }
    }

    #[test]
    fn the_pass_and_the_fail_are_never_one_inside_the_other_in_any_language() {
        for lang in Lang::ALL {
            let (pass, fail) = (Key::Pass.text(lang), Key::Fail.text(lang));
            assert!(!fail.contains(pass) && !pass.contains(fail), "{lang:?}");
        }
    }

    #[test]
    fn a_program_that_moves_nothing_has_no_box() {
        let (v, l, a) = parts(b"G21\nM2\n");
        let none = Interlocks::default();
        let en = render(&input_of(&v, &l, &a, &none), Lang::En);
        assert!(en.contains("Work box: nothing moves"), "{en}");
        assert!(en.contains("FAILS"), "{en}");
    }
}
