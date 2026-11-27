// EH3D-PHOENIX-LASER - what an alarm or an error number of a GRBL controller means.
// Copyright (C) 2026 JuanenRac (Electro Hobby 3D). GPL-3.0-or-later.
//! A GRBL controller answers a problem with `ALARM:n` or `error:n`. This table says in the eight languages what the common ones mean, in words of this program. It only reads the text a
//! person gives: nothing here talks to a machine, unlocks an alarm or sends anything.

use crate::lang::Lang;

/// The kind of answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Answer {
    Alarm,
    Error,
}

/// A number and its sentence in each language, in the order of [`Lang::ALL`].
pub type Entry = (u32, [&'static str; 8]);

pub const ALARMS: &[Entry] = &[
    (1, [
        "A limit switch was hit while moving. The position is probably lost: home again before any job.",
        "Se pulsó un final de carrera en movimiento. La posición probablemente se ha perdido: haz el origen antes de cualquier trabajo.",
        "Beim Fahren wurde ein Endschalter ausgelöst. Die Position ist wahrscheinlich verloren: vor jedem Auftrag neu referenzieren.",
        "Un fin de course a été touché en mouvement. La position est probablement perdue : refaites la prise d'origine avant tout travail.",
        "Un finecorsa è scattato durante il movimento. La posizione è probabilmente persa: rifare l'azzeramento prima di ogni lavoro.",
        "移動中にリミットスイッチが作動しました。位置はおそらく失われています：作業の前に原点復帰してください。",
        "运动中触发了限位开关。位置很可能已丢失：任何作业前请重新回零。",
        "Um fim de curso foi acionado em movimento. A posição está provavelmente perdida: refaça a origem antes de qualquer trabalho.",
    ]),
    (2, [
        "A move would go past the travel of the machine (soft limit). The position is kept: check the job and its origin before unlocking.",
        "Un movimiento saldría del recorrido de la máquina (límite por software). La posición se conserva: revisa el trabajo y su origen antes de desbloquear.",
        "Eine Bewegung ginge über den Verfahrweg hinaus (Software-Endlage). Die Position bleibt: Auftrag und Nullpunkt vor dem Entsperren prüfen.",
        "Un mouvement dépasserait la course de la machine (limite logicielle). La position est conservée : vérifiez le travail et son origine avant de déverrouiller.",
        "Un movimento uscirebbe dalla corsa della macchina (limite software). La posizione è mantenuta: controllare il lavoro e la sua origine prima di sbloccare.",
        "移動が機械の可動範囲を超えます（ソフトリミット）。位置は保持されています：解除する前に作業と原点を確認してください。",
        "运动将超出机器行程（软限位）。位置保持：解锁前请检查作业及其原点。",
        "Um movimento sairia do curso da máquina (limite por software). A posição mantém-se: verifique o trabalho e a origem antes de desbloquear.",
    ]),
    (3, [
        "The controller was reset while moving. The position is probably lost: home again.",
        "El controlador se reinició en movimiento. La posición probablemente se ha perdido: haz el origen de nuevo.",
        "Die Steuerung wurde während der Fahrt zurückgesetzt. Die Position ist wahrscheinlich verloren: neu referenzieren.",
        "Le contrôleur a été réinitialisé en mouvement. La position est probablement perdue : refaites la prise d'origine.",
        "Il controllore è stato reimpostato durante il movimento. La posizione è probabilmente persa: rifare l'azzeramento.",
        "移動中にコントローラがリセットされました。位置はおそらく失われています：原点復帰してください。",
        "控制器在运动中被复位。位置很可能已丢失：请重新回零。",
        "O controlador foi reiniciado em movimento. A posição está provavelmente perdida: refaça a origem.",
    ]),
    (4, [
        "Probing failed: the probe was already touching, or not as expected, before it started.",
        "Falló el sondeo: la sonda ya estaba tocando, o no como se esperaba, antes de empezar.",
        "Antasten fehlgeschlagen: der Taster war schon vor dem Start nicht im erwarteten Zustand.",
        "Palpage échoué : le palpeur n'était pas dans l'état attendu avant de commencer.",
        "Tastatura fallita: il tastatore non era nello stato previsto prima di iniziare.",
        "プローブに失敗しました：開始前のプローブの状態が想定と異なります。",
        "探测失败：开始前探针状态与预期不符。",
        "A sondagem falhou: a sonda não estava no estado esperado antes de começar.",
    ]),
    (5, [
        "Probing failed: the probe did not touch within the distance given.",
        "Falló el sondeo: la sonda no tocó dentro de la distancia dada.",
        "Antasten fehlgeschlagen: der Taster berührte innerhalb der Strecke nichts.",
        "Palpage échoué : le palpeur n'a rien touché sur la distance donnée.",
        "Tastatura fallita: il tastatore non ha toccato entro la distanza data.",
        "プローブに失敗しました：指定距離内で接触しませんでした。",
        "探测失败：在给定距离内未接触。",
        "A sondagem falhou: a sonda não tocou dentro da distância indicada.",
    ]),
    (6, [
        "Homing failed: the controller was reset during it.",
        "Falló la búsqueda de origen: el controlador se reinició durante ella.",
        "Referenzfahrt fehlgeschlagen: die Steuerung wurde dabei zurückgesetzt.",
        "Prise d'origine échouée : le contrôleur a été réinitialisé pendant celle-ci.",
        "Azzeramento fallito: il controllore è stato reimpostato nel mentre.",
        "原点復帰に失敗しました：途中でコントローラがリセットされました。",
        "回零失败：过程中控制器被复位。",
        "A procura da origem falhou: o controlador foi reiniciado entretanto.",
    ]),
    (7, [
        "Homing failed: the safety door was opened during it.",
        "Falló la búsqueda de origen: se abrió la puerta de seguridad durante ella.",
        "Referenzfahrt fehlgeschlagen: die Schutztür wurde dabei geöffnet.",
        "Prise d'origine échouée : la porte de sécurité a été ouverte pendant celle-ci.",
        "Azzeramento fallito: la porta di sicurezza è stata aperta nel mentre.",
        "原点復帰に失敗しました：途中で安全扉が開きました。",
        "回零失败：过程中安全门被打开。",
        "A procura da origem falhou: a porta de segurança foi aberta entretanto.",
    ]),
    (8, [
        "Homing failed: pulling off did not release the limit switch.",
        "Falló la búsqueda de origen: al retirarse no se soltó el final de carrera.",
        "Referenzfahrt fehlgeschlagen: beim Freifahren löste der Endschalter nicht.",
        "Prise d'origine échouée : le recul n'a pas libéré le fin de course.",
        "Azzeramento fallito: l'arretramento non ha liberato il finecorsa.",
        "原点復帰に失敗しました：離れる動作でリミットスイッチが解除されませんでした。",
        "回零失败：回退后限位开关仍未释放。",
        "A procura da origem falhou: o recuo não libertou o fim de curso.",
    ]),
    (9, [
        "Homing failed: no limit switch was found within the search distance.",
        "Falló la búsqueda de origen: no se encontró el final de carrera en la distancia de búsqueda.",
        "Referenzfahrt fehlgeschlagen: innerhalb des Suchwegs wurde kein Endschalter gefunden.",
        "Prise d'origine échouée : aucun fin de course trouvé sur la distance de recherche.",
        "Azzeramento fallito: nessun finecorsa trovato entro la distanza di ricerca.",
        "原点復帰に失敗しました：探索距離内にリミットスイッチが見つかりません。",
        "回零失败：在搜索距离内未找到限位开关。",
        "A procura da origem falhou: nenhum fim de curso encontrado na distância de procura.",
    ]),
];

pub const ERRORS: &[Entry] = &[
    (1, [
        "A word of G-code must be a letter and a number; the letter is missing.",
        "Una palabra de G-code es una letra y un número; falta la letra.",
        "Ein G-Code-Wort besteht aus Buchstabe und Zahl; der Buchstabe fehlt.",
        "Un mot de G-code est une lettre et un nombre ; la lettre manque.",
        "Una parola di G-code è una lettera e un numero; manca la lettera.",
        "G コードの語は文字と数値です。文字がありません。",
        "G 代码字由字母和数字组成；缺少字母。",
        "Uma palavra de G-code é uma letra e um número; falta a letra.",
    ]),
    (2, [
        "A number is missing or is not written in a form the controller reads.",
        "Falta un número o no está escrito de una forma que el controlador lea.",
        "Eine Zahl fehlt oder ist nicht lesbar geschrieben.",
        "Un nombre manque ou n'est pas écrit sous une forme lisible.",
        "Manca un numero o non è scritto in una forma leggibile.",
        "数値がないか、コントローラが読めない形式です。",
        "缺少数值，或数值格式无法读取。",
        "Falta um número ou não está escrito de forma legível pelo controlador.",
    ]),
    (3, [
        "A command that begins with $ is not known or not supported.",
        "Un comando que empieza por $ no se conoce o no se admite.",
        "Ein Befehl mit $ ist unbekannt oder wird nicht unterstützt.",
        "Une commande commençant par $ est inconnue ou non prise en charge.",
        "Un comando che inizia con $ non è noto o non è supportato.",
        "$ で始まるコマンドが不明または未対応です。",
        "以 $ 开头的命令未知或不受支持。",
        "Um comando que começa por $ não é conhecido ou não é suportado.",
    ]),
    (8, [
        "That command of the controller can only be used while it is idle.",
        "Ese comando del controlador solo puede usarse cuando está en reposo.",
        "Dieser Befehl geht nur, während die Steuerung im Leerlauf ist.",
        "Cette commande ne peut être utilisée que contrôleur au repos.",
        "Quel comando si può usare solo a controllore fermo.",
        "このコマンドはコントローラが待機中のときだけ使えます。",
        "该命令只能在控制器空闲时使用。",
        "Esse comando só pode ser usado com o controlador em repouso.",
    ]),
    (9, [
        "G-code is locked out while an alarm or a jog is active.",
        "El G-code está bloqueado mientras hay una alarma o un movimiento manual.",
        "G-Code ist gesperrt, solange ein Alarm oder ein Handfahren aktiv ist.",
        "Le G-code est bloqué tant qu'une alarme ou un déplacement manuel est actif.",
        "Il G-code è bloccato finché c'è un allarme o un movimento manuale.",
        "アラームまたはジョグ中は G コードを受け付けません。",
        "报警或点动期间 G 代码被锁定。",
        "O G-code está bloqueado enquanto houver um alarme ou um movimento manual.",
    ]),
    (11, [
        "A line is longer than the controller takes; it was not run.",
        "Una línea es más larga de lo que admite el controlador; no se ejecutó.",
        "Eine Zeile ist länger, als die Steuerung annimmt; sie wurde nicht ausgeführt.",
        "Une ligne est plus longue que ce que le contrôleur accepte ; elle n'a pas été exécutée.",
        "Una riga è più lunga di quanto il controllore accetta; non è stata eseguita.",
        "行がコントローラの上限より長いため、実行されませんでした。",
        "某行超过控制器可接受的长度；未执行。",
        "Uma linha é mais longa do que o controlador aceita; não foi executada.",
    ]),
    (13, [
        "The safety door was seen open.",
        "Se detectó la puerta de seguridad abierta.",
        "Die Schutztür wurde als offen erkannt.",
        "La porte de sécurité a été détectée ouverte.",
        "La porta di sicurezza è stata rilevata aperta.",
        "安全扉が開いていることを検出しました。",
        "检测到安全门打开。",
        "A porta de segurança foi detetada aberta.",
    ]),
    (15, [
        "A manual movement would go past the travel of the machine.",
        "Un movimiento manual saldría del recorrido de la máquina.",
        "Ein Handfahren ginge über den Verfahrweg hinaus.",
        "Un déplacement manuel dépasserait la course de la machine.",
        "Un movimento manuale uscirebbe dalla corsa della macchina.",
        "ジョグ移動が機械の可動範囲を超えます。",
        "点动目标超出机器行程。",
        "Um movimento manual sairia do curso da máquina.",
    ]),
    (17, [
        "The laser mode needs an output of variable power (PWM).",
        "El modo láser necesita una salida de potencia variable (PWM).",
        "Der Lasermodus braucht einen Ausgang mit variabler Leistung (PWM).",
        "Le mode laser demande une sortie à puissance variable (PWM).",
        "La modalità laser richiede un'uscita a potenza variabile (PWM).",
        "レーザーモードには可変出力（PWM）が必要です。",
        "激光模式需要可变功率输出（PWM）。",
        "O modo laser precisa de uma saída de potência variável (PWM).",
    ]),
    (20, [
        "A G-code command is not supported or not valid.",
        "Un comando G-code no se admite o no es válido.",
        "Ein G-Code-Befehl wird nicht unterstützt oder ist ungültig.",
        "Une commande G-code n'est pas prise en charge ou n'est pas valide.",
        "Un comando G-code non è supportato o non è valido.",
        "未対応または無効な G コードコマンドです。",
        "存在不支持或无效的 G 代码命令。",
        "Um comando G-code não é suportado ou não é válido.",
    ]),
    (22, [
        "A feed (F) has not been set before a move that needs it.",
        "No se ha fijado la velocidad (F) antes de un movimiento que la necesita.",
        "Vor einer Bewegung, die ihn braucht, wurde kein Vorschub (F) gesetzt.",
        "Aucune avance (F) n'a été donnée avant un mouvement qui en a besoin.",
        "Non è stato impostato l'avanzamento (F) prima di un movimento che lo richiede.",
        "送り速度（F）が必要な移動の前に指定されていません。",
        "需要进给速度的运动之前未设置 F。",
        "Não foi definida a velocidade (F) antes de um movimento que a exige.",
    ]),
    (33, [
        "The target of a movement is not valid.",
        "El destino de un movimiento no es válido.",
        "Das Ziel einer Bewegung ist ungültig.",
        "La cible d'un mouvement n'est pas valide.",
        "La destinazione di un movimento non è valida.",
        "移動先が無効です。",
        "运动目标无效。",
        "O destino de um movimento não é válido.",
    ]),
];

/// Reads `ALARM:2`, `alarm 2`, `error:11` or `Error 11`.
pub fn read_answer(text: &str) -> Option<(Answer, u32)> {
    let lower = text.trim().to_ascii_lowercase();
    let (answer, rest) = if let Some(rest) = lower.strip_prefix("alarm") {
        (Answer::Alarm, rest)
    } else {
        (Answer::Error, lower.strip_prefix("error")?)
    };
    let rest = rest.trim_start_matches([':', ' ']);
    if rest.is_empty() || !rest.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    rest.parse().ok().map(|n| (answer, n))
}

/// The sentence for an answer of the controller; a number that is not in the table is an error, never a guess.
pub fn explain(text: &str, lang: Lang) -> Result<&'static str, String> {
    let (answer, number) =
        read_answer(text).ok_or_else(|| format!("'{}' is not ALARM:n or error:n", text.trim()))?;
    let table = match answer {
        Answer::Alarm => ALARMS,
        Answer::Error => ERRORS,
    };
    let i = Lang::ALL.iter().position(|l| *l == lang).unwrap_or(0);
    table
        .iter()
        .find(|(n, _)| *n == number)
        .map(|(_, s)| s[i])
        .ok_or_else(|| {
            format!(
                "'{}' is not in the table of this program: look in the documents of the controller",
                text.trim()
            )
        })
}
