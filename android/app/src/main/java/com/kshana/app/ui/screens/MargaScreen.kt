package com.kshana.app.ui.screens

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.OutlinedTextFieldDefaults
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateMapOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontStyle
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.kshana.app.data.BirthProfile
import com.kshana.app.data.EngineSettings
import com.kshana.app.data.Place
import com.kshana.app.engine.EngineProvider
import com.kshana.app.engine.Instants
import com.kshana.app.ui.sky.KshanaFonts
import com.kshana.app.ui.theme.KshanaColors
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import uniffi.kshana_ffi.ArgalaRecord
import uniffi.kshana_ffi.AshtamangalaRecord
import uniffi.kshana_ffi.ChidraRecord
import uniffi.kshana_ffi.Engine
import uniffi.kshana_ffi.ExactInstant
import uniffi.kshana_ffi.JaiminiRecord
import uniffi.kshana_ffi.KarakamsaRecord
import uniffi.kshana_ffi.PadaRecord
import uniffi.kshana_ffi.PrasnaSphutaRecord
import uniffi.kshana_ffi.SandhiRecord
import uniffi.kshana_ffi.SphutaRecord
import uniffi.kshana_ffi.SphutaReportRecord
import uniffi.kshana_ffi.VimshopakaRecord
import uniffi.kshana_ffi.YogiRecord
import uniffi.kshana_ffi.ashtamangalaNumber
import java.time.ZoneId

/* ------------------------------------------------------------------------------------
 * Prasna Marga: the Kerala instrument. Five parts, one open at a time, each computed
 * only when opened and kept for the life of the screen. Every number comes from the
 * engine; every sentence under a number is the text's, shown as the text's.
 * ---------------------------------------------------------------------------------- */

private enum class MargaPart(val chip: String) {
    SPHUTAS("SPHUTAS"),
    ASHTAMANGALA("ASHTAMANGALA"),
    JAIMINI("JAIMINI"),
    SANDHI("SANDHI"),
    VIMSHOPAKA("VIMSHOPAKA"),
}

private val MG_SIGN_NAMES = listOf(
    "Aries", "Taurus", "Gemini", "Cancer", "Leo", "Virgo", "Libra", "Scorpio", "Sagittarius", "Capricorn", "Aquarius", "Pisces",
)

/** The prasna sphutas with the instant they were cast for. */
private class ArudhaCast(val at: ExactInstant, val record: PrasnaSphutaRecord)

/** What each part has already computed, so re-opening it costs nothing. */
private class MargaCache {
    /** Sphuta reports by "birth" or "now:<tai seconds>". */
    val sphutas = mutableStateMapOf<String, SphutaReportRecord>()
    /** Prasna sphutas by Arudha sign (0 = Aries), each cast for the instant it was first asked. */
    val arudha = mutableStateMapOf<Int, ArudhaCast>()
    var jaimini by mutableStateOf<JaiminiRecord?>(null)
    /** Sandhi windows by rule: "sub" or "eighth". */
    val sandhi = mutableStateMapOf<String, List<SandhiRecord>>()
    var runningLord by mutableStateOf<String?>(null)
    val chidra = mutableStateMapOf<String, List<ChidraRecord>>()
    var vimshopaka by mutableStateOf<List<VimshopakaRecord>?>(null)
}

/** D°MM′SS″ within the sign, whole seconds. */
private fun mgDms(lon: Double): String {
    val inSign = norm360(lon) % 30.0
    val total = (inSign * 3600.0).toInt()
    return "%d°%02d′%02d″".format(total / 3600, (total / 60) % 60, total % 60)
}

private fun mgSignOf(lon: Double): String = MG_SIGN_NAMES[signIndex(lon)]

private fun mgSignName(i: Int): String = MG_SIGN_NAMES[((i % 12) + 12) % 12]


private fun mgSigned(n: Int): String = when {
    n > 0 -> "+$n"
    n < 0 -> "−${-n}"
    else -> "0"
}

private fun mgList(names: List<String>): String = if (names.isEmpty()) "—" else names.joinToString(", ")

private fun mgGlyph(name: String): String = PLANET_GLYPHS[name] ?: ""

/** The Gulika convention name the engine prints, as a sentence. */
private fun mgGulikaConvention(code: String, night: Boolean): String {
    val part = if (night) "the night's" else "the day's"
    return when (code) {
        "JhoraStart" -> "the sidereal ascendant at the start of Saturn's part among $part eight parts (Jagannatha Hora's default)"
        "KeralaMandi" -> "the sidereal ascendant at the Kerala thirty-ghati Mandi moment of $part parts"
        else -> "$code, $part parts"
    }
}

/** The same wall-clock time shifted by whole years in the zone. */
private fun mgPlusYears(at: ExactInstant, zone: ZoneId, years: Long): ExactInstant =
    Instants.fromInstant(Instants.toLocal(at, zone).plusYears(years).toInstant())

/**
 * Runs [compute] off the main thread once per [key] unless [cached] is already there,
 * handing the result to [onLoaded]; returns the error, if any.
 */
@Composable
private fun <T : Any> mgLoad(key: Any?, cached: T?, onLoaded: (T) -> Unit, compute: suspend (Engine) -> T): String? {
    val context = LocalContext.current
    var error by remember(key) { mutableStateOf<String?>(null) }
    LaunchedEffect(key) {
        if (cached != null) return@LaunchedEffect
        try {
            val engine = EngineProvider.get(context)
            val r = withContext(Dispatchers.Default) { compute(engine) }
            onLoaded(r)
            error = null
        } catch (e: CancellationException) {
            throw e
        } catch (e: Exception) {
            error = e.message ?: e.toString()
        }
    }
    return error
}

/**
 * The Kerala instrument: the Prasna Marga sphutas (the longevity reckoning is never shown), the
 * Ashtamangala number with the prasna Arudha, the Jaimini extras, the Vimshottari
 * sandhi with the chidra lords, and the Vimshopaka bala. One part open at a time.
 */
@Composable
fun MargaScreen(profile: BirthProfile, here: Place, settings: EngineSettings, onBack: () -> Unit) {
    val zone = remember { ZoneId.systemDefault() }
    val cache = remember(profile, settings, here, com.kshana.app.data.Lang.code) { MargaCache() }
    var part by remember(profile, settings) { mutableStateOf(MargaPart.SPHUTAS) }

    Column(
        Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(horizontal = 24.dp).padding(top = 48.dp, bottom = 32.dp),
        verticalArrangement = Arrangement.spacedBy(10.dp),
    ) {
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween, verticalAlignment = Alignment.CenterVertically) {
            Text("PRASNA MARGA", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
            QuietButton("BACK", onClick = onBack)
        }
        Text("The Kerala instrument", style = MaterialTheme.typography.displayMedium, color = KshanaColors.Text)
        Text(
            "Sphutas, Ashtamangala, Jaimini, sandhi, Vimshopaka · ${profile.name} · ${profile.placeName}",
            style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted,
        )

        Row(
            Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()).padding(vertical = 4.dp),
            horizontalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            for (p in MargaPart.entries) Chip(p.chip, selected = part == p, onClick = { part = p })
        }

        when (part) {
            MargaPart.SPHUTAS -> SphutaPart(profile, here, settings, zone, cache)
            MargaPart.ASHTAMANGALA -> AshtamangalaPart(here, settings, zone, cache)
            MargaPart.JAIMINI -> JaiminiPart(profile, settings, cache)
            MargaPart.SANDHI -> SandhiPart(profile, settings, zone, cache)
            MargaPart.VIMSHOPAKA -> VimshopakaPart(profile, settings, cache)
        }

        Text(
            "Prasna Marga (Raman's edition), BPHS chapters 8, 43 and 44, the Jaimini Sutras; conventions stated in the engine's docs.",
            fontFamily = KshanaFonts.mono, fontSize = 10.sp, lineHeight = 15.sp, color = KshanaColors.Muted,
            modifier = Modifier.padding(vertical = 8.dp),
        )
    }
}

/* ------------------------------------------------------------------------------------
 * Shared pieces.
 * ---------------------------------------------------------------------------------- */

@Composable
private fun MgLoading(text: String) {
    Text(text, style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted)
}

/** A night card with a hairline. */
@Composable
private fun MgCard(content: @Composable () -> Unit) {
    val shape = RoundedCornerShape(8.dp)
    Column(
        Modifier.fillMaxWidth().clip(shape).background(KshanaColors.Surface).border(1.dp, KshanaColors.Line, shape).padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(6.dp),
    ) {
        content()
    }
}

@Composable
private fun MgCardTitle(title: String, tag: String) {
    Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
        Text(title, fontFamily = KshanaFonts.serif, fontSize = 20.sp, color = KshanaColors.Text, modifier = Modifier.weight(1f))
        Text(tag, style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted)
    }
}

/** A serif sentence in the house voice. */
@Composable
private fun MgSentence(text: String, colour: Color = KshanaColors.Text) {
    Text(text, fontFamily = KshanaFonts.serif, fontSize = 18.sp, lineHeight = 24.sp, color = colour)
}

/** The one-line muted footer each part ends with: what is exact, what is the text's. */
@Composable
private fun MgFooter(text: String) {
    Text(
        text,
        fontFamily = KshanaFonts.serif, fontStyle = FontStyle.Italic, fontSize = 14.sp, lineHeight = 20.sp, color = KshanaColors.Muted,
        modifier = Modifier.padding(top = 10.dp),
    )
}

@Composable
private fun MgBullet(text: String, colour: Color = KshanaColors.TextSoft) {
    Row(verticalAlignment = Alignment.Top) {
        Text("•", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Gold, modifier = Modifier.width(16.dp))
        Text(sentence(text), style = MaterialTheme.typography.bodyMedium, color = colour, modifier = Modifier.weight(1f))
    }
}

/** The text's words under a toggle, closed by default; nothing drawn when there are none. */
@Composable
private fun MgWords(words: List<String>, key: Any, label: String = "THE WORDS") {
    if (words.isEmpty()) return
    var open by remember(key) { mutableStateOf(false) }
    Row(
        Modifier.fillMaxWidth().height(32.dp).clickable { open = !open },
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.SpaceBetween,
    ) {
        Text(label, style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
        Text(if (open) "hide" else "show", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted)
    }
    if (open) for (w in words) MgBullet(w)
}

/** One sensitive point: name, sign and degree, nakshatra and pada, the two lords, the words. */
@Composable
private fun SphutaRow(s: SphutaRecord, key: Any = s.name) {
    HorizontalDivider(color = KshanaColors.LineSoft)
    Column(Modifier.fillMaxWidth().padding(vertical = 6.dp), verticalArrangement = Arrangement.spacedBy(2.dp)) {
        Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
            Text(s.name, fontFamily = KshanaFonts.serif, fontSize = 18.sp, color = KshanaColors.Text, modifier = Modifier.weight(1f))
            Text("${s.signName} ${mgDms(s.longitude)}", fontFamily = KshanaFonts.mono, fontSize = 12.sp, color = KshanaColors.Sky)
        }
        Text("${s.nakshatraName} · pada ${s.pada.toInt()}", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.TextSoft)
        Text(
            "sign lord ${mgGlyph(s.lord)} ${s.lord} · star lord ${mgGlyph(s.starLord)} ${s.starLord}",
            style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted,
        )
        MgWords(s.words, key)
    }
}

/* ------------------------------------------------------------------------------------
 * 1. Sphutas.
 * ---------------------------------------------------------------------------------- */

@Composable
private fun SphutaPart(profile: BirthProfile, here: Place, settings: EngineSettings, zone: ZoneId, cache: MargaCache) {
    var nowAt by remember(profile, settings) { mutableStateOf<ExactInstant?>(null) }
    val birth = remember(profile) { profile.birthInstant() }
    val at = nowAt ?: birth
    val isNow = nowAt != null
    val key = if (isNow) "now:${at.taiSeconds}" else "birth"
    val lat = if (isNow) here.latitude else profile.latitude
    val lon = if (isNow) here.longitude else profile.longitude
    val placeName = if (isNow) here.name else profile.placeName

    val error = mgLoad(key, cache.sphutas[key], { cache.sphutas[key] = it }) { engine ->
        engine.prasnaMargaSphutas(at, lat, lon, settings.ayanamsa, com.kshana.app.data.Lang.code)
    }
    val rec = cache.sphutas[key]

    SectionLabel("Sphutas")
    Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(8.dp), verticalAlignment = Alignment.CenterVertically) {
        Chip("BIRTH", selected = !isNow, onClick = { nowAt = null })
        Chip("NOW", selected = isNow, onClick = { nowAt = Instants.now() })
        Spacer(Modifier.weight(1f))
    }
    Text(
        "${localText(at, DATE_TIME_S, zone)} · $placeName",
        fontFamily = KshanaFonts.mono, fontSize = 12.sp, color = KshanaColors.Sky,
    )
    ErrorLine(error)
    if (rec == null) {
        if (error == null) MgLoading("Summing the lagna, the Moon and Gulika…")
    } else {
        MgCard {
            MgCardTitle("Gulika", if (rec.gulikaNight) "NIGHT" else "DAY")
            MonoRow("Gulika", "${mgSignOf(rec.gulikaSidereal)} ${mgDms(rec.gulikaSidereal)}")
            MonoRow("Saturn's part begins", localText(rec.gulikaAt, DATE_TIME_S, zone), KshanaColors.TextSoft)
            Text(
                "Gulika is ${mgGulikaConvention(rec.gulikaConvention, rec.gulikaNight)}, the eight parts lorded in weekday order from the weekday lord (BPHS 3.66–67).",
                style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted,
            )
        }

        SectionLabel("The eleven points")
        for (s in rec.sphutas) SphutaRow(s, "$key:${s.name}")

        SectionLabel("Yogi point")
        YogiCard(rec.yogi, key)

        SectionLabel("Bhrigu Bindu")
        SphutaRow(rec.bhriguBindu, "$key:bhrigu")
        Text(
            "The midpoint of the Moon and Rahu on the shorter arc.",
            style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted,
        )
    }
    MgFooter(
        "Exact: every sphuta is a sum or multiple of sidereal longitudes (Prasna Marga VIII, Beeja and Kshetra from Part II) and Gulika the ascendant at a solved instant of the solar day; the text's words: the sign, navamsa and nakshatra readings under each point, shown as readings and never as a verdict.",
    )
}

@Composable
private fun YogiCard(y: YogiRecord, key: String) {
    MgCard {
        MgCardTitle("Yogi point", "SUN + MOON + 93°20′")
        MonoRow("Point", "${y.point.signName} ${mgDms(y.point.longitude)}")
        MonoRow("Nakshatra", "${y.point.nakshatraName} · pada ${y.point.pada.toInt()}", KshanaColors.TextSoft)
        HorizontalDivider(color = KshanaColors.LineSoft)
        MonoRow("Yogi · star lord", "${mgGlyph(y.yogi)} ${y.yogi}", KshanaColors.Gold)
        MonoRow("Sahayogi · sign lord", "${mgGlyph(y.sahayogi)} ${y.sahayogi}")
        MonoRow("Avayogi · 186°40′ on", "${mgGlyph(y.avayogi)} ${y.avayogi}")
        MonoRow("Avayogi point", "${mgSignOf(y.avayogiPoint)} ${mgDms(y.avayogiPoint)}", KshanaColors.TextSoft)
        Text(
            "The Avayogi is the lord of the nakshatra 186°40′ on, the sixth from the Yogi's; a few authors take that point's sign lord instead, which is not done here.",
            style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted,
        )
        MgWords(y.words + y.point.words, "$key:yogi")
    }
}

/* ------------------------------------------------------------------------------------
 * 2. Ashtamangala.
 * ---------------------------------------------------------------------------------- */

@Composable
private fun AshtamangalaPart(here: Place, settings: EngineSettings, zone: ZoneId, cache: MargaCache) {
    var typed by remember { mutableStateOf("") }
    val number = typed.toIntOrNull()
    val valid = number != null && number in 1..999
    val read: AshtamangalaRecord? = remember(number, com.kshana.app.data.Lang.code) { if (number != null && number in 1..999) ashtamangalaNumber(number.toUInt(), com.kshana.app.data.Lang.code) else null }

    SectionLabel("Ashtamangala")
    MgSentence("One hundred and eight cowries in three heaps, each counted off in eights; the three remainders make the number.")
    OutlinedTextField(
        value = typed,
        onValueChange = { typed = it.filter(Char::isDigit).take(3) },
        label = { Text("the number, 1 to 999 (digits 1–8 are the cowrie remainders)", style = MaterialTheme.typography.labelSmall) },
        singleLine = true,
        keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
        textStyle = TextStyle(fontFamily = KshanaFonts.mono, fontSize = 18.sp, color = KshanaColors.Text),
        colors = mgFieldColors(),
        modifier = Modifier.fillMaxWidth(),
    )
    if (typed.isNotEmpty() && !valid) {
        Text("The number must be between 1 and 999.", style = MaterialTheme.typography.bodySmall, color = KshanaColors.Sky)
    }

    if (read == null) {
        if (typed.isEmpty()) MgLoading("Type the number the cowries gave.")
    } else {
        AshtamangalaBody(read)

        // The Kerala prasna's sphutas for the moment the number was read, the gold piece on the Arudha sign.
        val sign = read.arudhaSign.toInt()
        val error = mgLoad(sign, cache.arudha[sign], { cache.arudha[sign] = it }) { engine ->
            val at = Instants.now()
            ArudhaCast(at, engine.prasnaArudhaSphutas(at, here.latitude, here.longitude, settings.ayanamsa, sign.toUByte(), com.kshana.app.data.Lang.code))
        }
        SectionLabel("The prasna now")
        ErrorLine(error)
        val cast = cache.arudha[sign]
        if (cast == null) {
            if (error == null) MgLoading("Casting the lagna of this moment at ${here.name}…")
        } else {
            val p = cast.record
            Text(
                "${localText(cast.at, DATE_TIME_S, zone)} · ${here.name} · gold piece on ${mgSignName(p.arudhaSign.toInt())}",
                fontFamily = KshanaFonts.mono, fontSize = 12.sp, color = KshanaColors.Sky,
            )
            SphutaRow(p.lagna, "arudha:$sign:lagna")
            SphutaRow(p.chandra, "arudha:$sign:chandra")
            SphutaRow(p.arudha, "arudha:$sign:arudha")
            MgCard {
                MgCardTitle("Veethi and Chatra", "FROM THE SUN")
                MonoRow("Veethi rasi", p.veethiSignName)
                MonoRow("Chatra rasi", p.chatraSignName)
                Text(
                    "The Veethi follows the Sun's sign (Taurus–Leo give Aries, Scorpio–Aquarius Gemini, the rest Taurus); the Chatra is counted from the Veethi as many signs as the lagna is from the Arudha, both ends counted.",
                    style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted,
                )
                MgWords(p.words, "arudha:$sign:words")
            }
        }
    }
    MgFooter(
        "Exact: the digits, their planets, odd and even, and the Veethi and Chatra counts (Prasna Marga IV and the Chatra table); practice, not stanza: the sign and nakshatra read from the remainders by twelve and twenty-seven, and the Arudha sphuta as the lagna's degree carried into the Arudha sign.",
    )
}

@Composable
private fun AshtamangalaBody(a: AshtamangalaRecord) {
    val digits = a.digits.map { it.toInt() }
    val labels = if (digits.size == 3) listOf("PAST", "PRESENT", "FUTURE") else digits.indices.map { "DIGIT ${it + 1}" }
    MgCard {
        MgCardTitle(a.number.toString(), "THE NUMBER")
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            for ((i, d) in digits.withIndex()) {
                val planet = a.planets.getOrNull(i)
                Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(2.dp)) {
                    Text(labels[i], style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted)
                    Text(d.toString(), fontFamily = KshanaFonts.mono, fontSize = 24.sp, color = if (planet != null) KshanaColors.Gold else KshanaColors.Muted)
                    Text(
                        if (planet != null) "${mgGlyph(planet)} $planet".trim() else "not a cowrie digit",
                        style = MaterialTheme.typography.bodyMedium, color = KshanaColors.TextSoft,
                    )
                    Text(
                        if (d % 2 == 1) "odd · good" else "even · bad",
                        fontFamily = KshanaFonts.mono, fontSize = 10.sp, color = KshanaColors.Muted,
                    )
                }
            }
        }
        HorizontalDivider(color = KshanaColors.LineSoft)
        MonoRow("Signals the sign", a.arudhaSignName)
        val nk = a.nakshatraName
        if (nk != null) MonoRow("Signals the nakshatra", nk)
        if (a.indications.isNotEmpty()) {
            HorizontalDivider(color = KshanaColors.LineSoft)
            for (w in a.indications) MgBullet(w)
        }
    }
}

@Composable
private fun mgFieldColors() = OutlinedTextFieldDefaults.colors(
    focusedTextColor = KshanaColors.Text,
    unfocusedTextColor = KshanaColors.Text,
    cursorColor = KshanaColors.Gold,
    focusedBorderColor = KshanaColors.Gold,
    unfocusedBorderColor = KshanaColors.Line,
    focusedLabelColor = KshanaColors.Gold,
    unfocusedLabelColor = KshanaColors.Muted,
    focusedContainerColor = KshanaColors.Surface,
    unfocusedContainerColor = KshanaColors.Surface,
)

/* ------------------------------------------------------------------------------------
 * 3. Jaimini.
 * ---------------------------------------------------------------------------------- */

@Composable
private fun JaiminiPart(profile: BirthProfile, settings: EngineSettings, cache: MargaCache) {
    val error = mgLoad("jaimini", cache.jaimini, { cache.jaimini = it }) { engine ->
        engine.jaiminiExtras(profile.birthInstant(), profile.latitude, profile.longitude, settings.ayanamsa, com.kshana.app.data.Lang.code)
    }
    SectionLabel("Jaimini")
    ErrorLine(error)
    val rec = cache.jaimini
    if (rec == null) {
        if (error == null) MgLoading("Finding the Atmakaraka's navamsa, the padas and the argala…")
    } else {
        KarakamsaCard(rec.karakamsa)
        PadaCard(rec.padaLagna, "ARUDHA OF THE 1ST")
        PadaCard(rec.upapada, "ARUDHA OF THE 12TH")

        SectionLabel("Argala on the twelve houses")
        Row(Modifier.fillMaxWidth()) {
            Text("H", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted, modifier = Modifier.width(24.dp))
            Text("SIGN", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted, modifier = Modifier.weight(1f))
            Text("BY", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted, modifier = Modifier.weight(1.2f))
            Text("OBSTRUCTED", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted, modifier = Modifier.weight(1.2f))
            Text("NET", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted, modifier = Modifier.width(36.dp))
        }
        for (a in rec.argala) ArgalaRow(a)
        Text(
            "Planets in the 2nd, 4th and 11th from a house intervene (argala); those in the 12th, 10th and 3rd obstruct; the 5th is secondary, obstructed by the 9th; malefics in the 3rd give an argala nothing obstructs. Net is the count on each side, Ketu counted in reverse.",
            style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted,
        )

        SectionLabel("Rasi drishti of the lagna")
        val lagnaSign = rec.argala.firstOrNull { it.house.toInt() == 1 }?.sign?.toInt()
        val drishti = lagnaSign?.let { ls -> rec.rasiDrishti.firstOrNull { it.sign.toInt() == ls } }
        if (drishti == null) {
            MgLoading("The lagna's sign could not be read from the argala table.")
        } else {
            val seen = drishti.aspects.map { mgSignName(it.toInt()) }
            MgSentence("${drishti.signName} aspects ${seen.joinToString(", ")}.")
            Text(
                "A movable sign aspects the fixed signs except the one next to it, a fixed sign the movable signs except the one next to it, a dual sign the other dual signs; the relation is mutual.",
                style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted,
            )
        }
    }
    MgFooter(
        "Exact: rasi drishti, the argala and its counts, the Atmakaraka's navamsa sign and the padas (Jaimini Sutras 1.1 and 1.2); the text's words: the sutra for the karakamsa sign, three of the twelve unverified and saying so, and the readings of the padas the schools draw.",
    )
}

@Composable
private fun KarakamsaCard(k: KarakamsaRecord) {
    MgCard {
        MgCardTitle("Karakamsa", "ATMAKARAKA'S NAVAMSA")
        Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
            Text("${mgGlyph(k.atmakaraka)} ${k.atmakaraka}".trim(), fontFamily = KshanaFonts.serif, fontSize = 20.sp, color = KshanaColors.Gold, modifier = Modifier.weight(1f))
            Text(mgDms(k.degreeInSign), fontFamily = KshanaFonts.mono, fontSize = 12.sp, color = KshanaColors.Sky)
        }
        MonoRow("Karakamsa sign", k.signName)
        MonoRow("Navamsa lagna", mgSignName(k.navamsaLagna.toInt()), KshanaColors.TextSoft)
        MonoRow("In the karakamsa", mgList(k.occupants), KshanaColors.TextSoft)
        if (k.words.isNotEmpty()) {
            HorizontalDivider(color = KshanaColors.LineSoft)
            for (w in k.words) MgBullet(w)
        }
    }
}

@Composable
private fun PadaCard(p: PadaRecord, tag: String) {
    MgCard {
        MgCardTitle(p.name, tag)
        Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
            Text(p.signName, fontFamily = KshanaFonts.serif, fontSize = 20.sp, color = KshanaColors.Text, modifier = Modifier.weight(1f))
            Text("house ${p.rasiHouse.toInt()}", fontFamily = KshanaFonts.mono, fontSize = 12.sp, color = KshanaColors.Sky)
        }
        MonoRow("Arudha of house", p.house.toInt().toString(), KshanaColors.TextSoft)
        MonoRow("Lord", "${mgGlyph(p.lord)} ${p.lord}".trim(), KshanaColors.Gold)
        HorizontalDivider(color = KshanaColors.LineSoft)
        MonoRow("2nd from it", mgList(p.second), KshanaColors.TextSoft)
        MonoRow("7th from it", mgList(p.seventh), KshanaColors.TextSoft)
        MonoRow("11th from it", mgList(p.eleventh), KshanaColors.TextSoft)
        MgWords(p.words, "pada:${p.name}")
    }
}

/** One house's argala: house, sign, by, obstructed by, net; the pairs and the words under a toggle. */
@Composable
private fun ArgalaRow(a: ArgalaRecord) {
    var open by remember(a.house) { mutableStateOf(false) }
    val net = a.net.toInt()
    val netColour = when {
        net > 0 -> KshanaColors.Gold
        net < 0 -> KshanaColors.Sky
        else -> KshanaColors.Muted
    }
    HorizontalDivider(color = KshanaColors.LineSoft)
    Column(Modifier.fillMaxWidth().clickable { open = !open }.padding(vertical = 5.dp), verticalArrangement = Arrangement.spacedBy(2.dp)) {
        Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.Top) {
            Text(a.house.toInt().toString(), fontFamily = KshanaFonts.mono, fontSize = 12.sp, color = KshanaColors.Muted, modifier = Modifier.width(24.dp))
            Text(a.signName, style = MaterialTheme.typography.bodyMedium, color = KshanaColors.TextSoft, modifier = Modifier.weight(1f))
            Text(mgList(a.by.map { PLANET_ABBR[it] ?: it }), fontFamily = KshanaFonts.mono, fontSize = 11.sp, color = KshanaColors.TextSoft, modifier = Modifier.weight(1.2f))
            Text(mgList(a.obstructedBy.map { PLANET_ABBR[it] ?: it }), fontFamily = KshanaFonts.mono, fontSize = 11.sp, color = KshanaColors.Muted, modifier = Modifier.weight(1.2f))
            Text(mgSigned(net), fontFamily = KshanaFonts.mono, fontSize = 12.sp, color = netColour, modifier = Modifier.width(36.dp))
        }
        if (open) {
            for (p in a.pairs) {
                Text(
                    "${if (p.secondary) "secondary " else ""}${p.argalaHouse.toInt()}th by ${mgList(p.by)} · ${p.virodhaHouse.toInt()}th obstructs by ${mgList(p.obstructedBy)} · net ${mgSigned(p.net.toInt())}",
                    fontFamily = KshanaFonts.mono, fontSize = 10.sp, lineHeight = 14.sp, color = KshanaColors.Muted,
                    modifier = Modifier.padding(start = 24.dp),
                )
            }
            for (w in a.words) MgBullet(w)
        } else if (a.words.isNotEmpty() || a.pairs.isNotEmpty()) {
            Text("tap for the words", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted, modifier = Modifier.padding(start = 24.dp))
        }
    }
}

/* ------------------------------------------------------------------------------------
 * 4. Sandhi.
 * ---------------------------------------------------------------------------------- */

@Composable
private fun SandhiPart(profile: BirthProfile, settings: EngineSettings, zone: ZoneId, cache: MargaCache) {
    var eighth by remember(profile, settings) { mutableStateOf(false) }
    val from = remember(profile, settings) { Instants.now() }
    val to = remember(from) { mgPlusYears(from, zone, 10) }
    val ruleKey = if (eighth) "eighth" else "sub"
    val birth = remember(profile) { profile.birthInstant() }

    val error = mgLoad("sandhi:$ruleKey", cache.sandhi[ruleKey], { cache.sandhi[ruleKey] = it }) { engine ->
        engine.dashaSandhiWindows(birth, profile.latitude, profile.longitude, settings.ayanamsa, from, to, if (eighth) 0.125 else null, com.kshana.app.data.Lang.code)
    }
    val windows = cache.sandhi[ruleKey]

    SectionLabel("Dasha sandhi")
    MgSentence("The junctions of the Vimshottari periods in the ten years ahead: the close of one lord and the opening of the next.")
    Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(8.dp), verticalAlignment = Alignment.CenterVertically) {
        Chip("SUB-PERIODS", selected = !eighth, onClick = { eighth = false })
        Chip("1/8", selected = eighth, onClick = { eighth = true })
        Spacer(Modifier.weight(1f))
    }
    Text(
        "${localText(from, DATE_ONLY, zone)} – ${localText(to, DATE_ONLY, zone)} · ${zone.id}",
        fontFamily = KshanaFonts.mono, fontSize = 12.sp, color = KshanaColors.Sky,
    )
    ErrorLine(error)
    if (windows == null) {
        if (error == null) MgLoading("Laying out the Vimshottari from the birth Moon…")
    } else {
        if (windows.isEmpty()) MgLoading("No junction falls in the ten years ahead.")
        for (w in windows) SandhiRow(w, zone)
    }

    // The chidra lords of the running mahadasha.
    SectionLabel("Chidra lords of the running dasha")
    val lordError = mgLoad("lord", cache.runningLord, { cache.runningLord = it }) { engine ->
        val running = runCatching {
            engine.dashaSystem(birth, profile.latitude, profile.longitude, settings.ayanamsa, "vimshottari", from)
                .running.firstOrNull { it.level.toInt() == 1 }?.label
        }.getOrNull()
        running ?: windows?.firstOrNull()?.fromLord ?: throw IllegalStateException("The running mahadasha could not be read.")
    }
    ErrorLine(lordError)
    val lord = cache.runningLord
    if (lord == null) {
        if (lordError == null) MgLoading("Reading the running mahadasha…")
    } else {
        val chidraError = mgLoad("chidra:$lord", cache.chidra[lord], { cache.chidra[lord] = it }) { engine ->
            engine.dashaChidraLords(birth, profile.latitude, profile.longitude, settings.ayanamsa, lord, com.kshana.app.data.Lang.code)
        }
        Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
            Text("${mgGlyph(lord)} $lord".trim(), fontFamily = KshanaFonts.serif, fontSize = 20.sp, color = KshanaColors.Gold, modifier = Modifier.weight(1f))
            Text("MAHADASHA", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted)
        }
        ErrorLine(chidraError)
        val chidra = cache.chidra[lord]
        if (chidra == null) {
            if (chidraError == null) MgLoading("Counting the 2nd, 7th and 12th, the Khara drekkana and the 64th navamsa…")
        } else {
            if (chidra.isEmpty()) MgLoading("No chidra lord reads from $lord here.")
            for (c in chidra) ChidraRow(c)
        }
    }
    MgFooter(
        "Exact: every boundary and window is Vimshottari arithmetic from the birth Moon with a 365.25-day year, the rule named in each window, and the chidra lords are the classical list with their reasons; convention: the texts fix no width for a sandhi, so the sub-period rule is structural and the eighth a Kerala habit offered as a parameter.",
    )
}

@Composable
private fun SandhiRow(w: SandhiRecord, zone: ZoneId) {
    val level = w.level.toInt()
    val startMillis = Instants.toLocal(w.start, zone).toInstant().toEpochMilli()
    HorizontalDivider(color = KshanaColors.LineSoft)
    Column(Modifier.fillMaxWidth().padding(vertical = 8.dp), verticalArrangement = Arrangement.spacedBy(3.dp)) {
        Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
            Text(
                "L$level",
                style = MaterialTheme.typography.labelSmall, color = if (level == 1) KshanaColors.Gold else KshanaColors.Muted,
                modifier = Modifier.border(1.dp, if (level == 1) KshanaColors.Gold else KshanaColors.Line, RoundedCornerShape(4.dp)).padding(horizontal = 6.dp, vertical = 3.dp),
            )
            Spacer(Modifier.width(10.dp))
            Text(
                "${mgGlyph(w.fromLord)} ${w.fromLord} → ${mgGlyph(w.toLord)} ${w.toLord}",
                fontFamily = KshanaFonts.serif, fontSize = if (level == 1) 20.sp else 17.sp, color = KshanaColors.Text, modifier = Modifier.weight(1f),
            )
            RemindButton(
                key = "sandhi:${w.boundary.taiSeconds}",
                atMillis = startMillis,
                title = "Dasha sandhi: ${w.fromLord} → ${w.toLord}",
                text = "the window opens now; the boundary is at ${localText(w.boundary, DATE_TIME_S, zone)}",
            )
        }
        Row(verticalAlignment = Alignment.CenterVertically) {
            Text("boundary", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted, modifier = Modifier.width(64.dp))
            Text(localText(w.boundary, DATE_TIME_S, zone), fontFamily = KshanaFonts.mono, fontSize = 12.sp, color = KshanaColors.Sky)
        }
        Row(verticalAlignment = Alignment.CenterVertically) {
            Text("window", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted, modifier = Modifier.width(64.dp))
            Text(
                "${localText(w.start, DATE_TIME, zone)} – ${localText(w.end, DATE_TIME, zone)}",
                fontFamily = KshanaFonts.mono, fontSize = 11.sp, color = KshanaColors.TextSoft,
            )
        }
        Text(w.rule, style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted)
    }
}

@Composable
private fun ChidraRow(c: ChidraRecord) {
    HorizontalDivider(color = KshanaColors.LineSoft)
    Row(Modifier.fillMaxWidth().padding(vertical = 5.dp), verticalAlignment = Alignment.Top) {
        Text("${mgGlyph(c.lord)} ${c.lord}".trim(), style = MaterialTheme.typography.bodyMedium, color = KshanaColors.TextSoft, modifier = Modifier.width(96.dp))
        Text(sentence(c.reason), style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted, modifier = Modifier.weight(1f))
    }
}

/* ------------------------------------------------------------------------------------
 * 5. Vimshopaka.
 * ---------------------------------------------------------------------------------- */

@Composable
private fun VimshopakaPart(profile: BirthProfile, settings: EngineSettings, cache: MargaCache) {
    val error = mgLoad("vimshopaka", cache.vimshopaka, { cache.vimshopaka = it }) { engine ->
        engine.vimshopakaBala(profile.birthInstant(), profile.latitude, profile.longitude, settings.ayanamsa, com.kshana.app.data.Lang.code)
    }
    SectionLabel("Vimshopaka bala")
    MgSentence("Each graha's strength out of twenty across the divisional charts, in the four schemes the text weighs.")
    ErrorLine(error)
    val rows = cache.vimshopaka
    if (rows == null) {
        if (error == null) MgLoading("Placing the seven grahas in sixteen vargas…")
    } else {
        Row(Modifier.fillMaxWidth()) {
            Text("GRAHA", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted, modifier = Modifier.weight(1.3f))
            Text("SHAD", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted, modifier = Modifier.weight(0.8f))
            Text("SAPTA", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted, modifier = Modifier.weight(0.8f))
            Text("DASHA", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted, modifier = Modifier.weight(0.8f))
            Text("SHODASHA", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted, modifier = Modifier.weight(1f))
        }
        for (v in rows) VimshopakaRow(v)
        Text(
            "Shadvarga six vargas, Saptavarga seven, Dashavarga ten, Shodashavarga sixteen; the grade is the word for the Shodashavarga score. Tap a row for each varga's sign and points.",
            style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted,
        )
    }
    MgFooter(
        "Exact: the four weighted sums over the vargas (BPHS 8), twenty in own sign and else by the compound friendship with the varga lord; convention: nothing extra for exaltation or debilitation, as the text awards none, and the grade's bands are the ones software prints.",
    )
}

@Composable
private fun VimshopakaRow(v: VimshopakaRecord) {
    var open by remember(v.graha) { mutableStateOf(false) }
    HorizontalDivider(color = KshanaColors.LineSoft)
    Column(Modifier.fillMaxWidth().clickable { open = !open }.padding(vertical = 5.dp), verticalArrangement = Arrangement.spacedBy(2.dp)) {
        Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
            Text("${mgGlyph(v.graha)} ${v.graha}".trim(), style = MaterialTheme.typography.bodyMedium, color = KshanaColors.TextSoft, modifier = Modifier.weight(1.3f))
            Text("%.2f".format(v.shadvarga), fontFamily = KshanaFonts.mono, fontSize = 11.sp, color = KshanaColors.Sky, modifier = Modifier.weight(0.8f))
            Text("%.2f".format(v.saptavarga), fontFamily = KshanaFonts.mono, fontSize = 11.sp, color = KshanaColors.Sky, modifier = Modifier.weight(0.8f))
            Text("%.2f".format(v.dashavarga), fontFamily = KshanaFonts.mono, fontSize = 11.sp, color = KshanaColors.Sky, modifier = Modifier.weight(0.8f))
            Text("%.2f".format(v.shodashavarga), fontFamily = KshanaFonts.mono, fontSize = 11.sp, color = KshanaColors.Gold, modifier = Modifier.weight(1f))
        }
        Text(v.grade, style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted)
        if (open) for (w in v.words) MgBullet(w, KshanaColors.Muted)
    }
}
