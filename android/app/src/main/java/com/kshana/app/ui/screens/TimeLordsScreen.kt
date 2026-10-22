package com.kshana.app.ui.screens

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
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
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
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
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.drawText
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.rememberTextMeasurer
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.kshana.app.data.BirthProfile
import com.kshana.app.data.EngineSettings
import com.kshana.app.engine.EngineProvider
import com.kshana.app.engine.Instants
import com.kshana.app.ui.theme.KshanaColors
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import uniffi.kshana_ffi.DashaSystemRecord
import uniffi.kshana_ffi.DirectionHitRecord
import uniffi.kshana_ffi.ExactInstant
import uniffi.kshana_ffi.FirdariaRecord
import uniffi.kshana_ffi.ProfectionRecord
import uniffi.kshana_ffi.ProgressedRecord
import uniffi.kshana_ffi.ZrRecord
import java.time.ZoneId
import com.kshana.app.ui.sky.KshanaFonts

private const val TL_DAY_S = 86_400L
private const val TL_TWO_YEARS_S = 731L * TL_DAY_S
private const val TL_LUNAR_SPAN_S = 100L * TL_DAY_S

private val TL_SIGN_NAMES = listOf(
    "Aries", "Taurus", "Gemini", "Cancer", "Leo", "Virgo", "Libra", "Scorpio", "Sagittarius", "Capricorn", "Aquarius", "Pisces",
)
private val TL_SIGN_GLYPHS = listOf("♈", "♉", "♊", "♋", "♌", "♍", "♎", "♏", "♐", "♑", "♒", "♓")

/** One section's result: the value, or why it could not be read. */
private class TlSection<T>(val value: T?, val error: String?)

/** Everything the screen computes for one instant, each section on its own. */
private class TimeLordsLoad(
    val dashas: TlSection<List<DashaSystemRecord>>,
    val profection: TlSection<ProfectionRecord>,
    val firdaria: TlSection<List<FirdariaRecord>>,
    val fortune: TlSection<List<ZrRecord>>,
    val spirit: TlSection<List<ZrRecord>>,
    val progressed: TlSection<ProgressedRecord>,
    val hits: TlSection<List<DirectionHitRecord>>,
    val lunar: TlSection<List<ExactInstant>>,
)

/** A labelled stretch on a strip. */
private class TlSeg(val label: String, val start: ExactInstant, val end: ExactInstant)

private fun <T> tlSection(block: () -> T): TlSection<T> = try {
    TlSection(block(), null)
} catch (e: CancellationException) {
    throw e
} catch (e: Exception) {
    TlSection(null, e.message ?: e.toString())
}

private fun tlWithin(start: ExactInstant, end: ExactInstant, at: ExactInstant): Boolean {
    val s = at.seconds()
    return s >= start.seconds() && s < end.seconds()
}

/** The same wall-clock time shifted by whole days or years in the zone. */
private fun shiftAt(at: ExactInstant, zone: ZoneId, days: Long, years: Long): ExactInstant =
    Instants.fromInstant(Instants.toLocal(at, zone).plusDays(days).plusYears(years).toInstant())

/** The sign names as the engine writes them on the Malayalam and Hindi chart pages. */
private val TL_SIGN_NAMES_ML = listOf(
    "മേടം", "ഇടവം", "മിഥുനം", "കർക്കടകം", "ചിങ്ങം", "കന്നി", "തുലാം", "വൃശ്ചികം", "ധനു", "മകരം", "കുംഭം", "മീനം",
)
private val TL_SIGN_NAMES_HI = listOf(
    "मेष", "वृषभ", "मिथुन", "कर्क", "सिंह", "कन्या", "तुला", "वृश्चिक", "धनु", "मकर", "कुंभ", "मीन",
)

private fun tlSignIndex(name: String): Int =
    listOf(TL_SIGN_NAMES, TL_SIGN_NAMES_ML, TL_SIGN_NAMES_HI).map { it.indexOf(name) }.firstOrNull { it >= 0 } ?: -1

private fun signGlyphFor(name: String): String {
    val i = tlSignIndex(name)
    return if (i >= 0) TL_SIGN_GLYPHS[i] else ""
}

private fun planetGlyphFor(name: String): String = PLANET_GLYPHS[name] ?: ""

/** A two- or three-letter label for a strip: planet abbreviation, sign short name, else the first three letters. */
private fun tlShortLabel(label: String): String {
    PLANET_ABBR[label]?.let { return it }
    val sign = tlSignIndex(label)
    if (sign >= 0) return SIGN_SHORT[sign]
    return label.take(3)
}

private fun tlLongitude(lon: Double): String = "${SIGN_SHORT[signIndex(lon)]} ${dms(lon)}"

/**
 * Time lords: every period system side by side at one instant. The five Vedic dashas,
 * then the profection, the firdaria, zodiacal releasing from Fortune and Spirit, the
 * secondary progressions with the next exact hits, and the next lunar returns.
 */
@Composable
fun TimeLordsScreen(profile: BirthProfile, settings: EngineSettings, onBack: () -> Unit) {
    val context = LocalContext.current
    val zone = remember { ZoneId.systemDefault() }
    var at by remember(profile, settings) { mutableStateOf(Instants.now()) }
    var isNow by remember(profile, settings) { mutableStateOf(true) }
    val cache = remember(profile, settings, com.kshana.app.data.Lang.code) { mutableStateMapOf<ExactInstant, TimeLordsLoad>() }
    var loadError by remember { mutableStateOf<String?>(null) }
    val notesOpen = remember(profile, settings) { mutableStateMapOf<String, Boolean>() }

    // Every system at the chosen instant, each on its own so one failure leaves the rest.
    LaunchedEffect(profile, settings, at, com.kshana.app.data.Lang.code) {
        if (cache.containsKey(at)) return@LaunchedEffect
        loadError = null
        try {
            val engine = EngineProvider.get(context)
            val birth = profile.birthInstant()
            val lat = profile.latitude
            val lon = profile.longitude
            val ay = settings.ayanamsa
            val a = at
            val r = withContext(Dispatchers.Default) {
                val dayAfter = ExactInstant(a.taiSeconds + TL_DAY_S, a.nanos)
                TimeLordsLoad(
                    dashas = tlSection { engine.dashaSystemsAt(birth, lat, lon, ay, a, com.kshana.app.data.Lang.code) },
                    profection = tlSection { engine.profections(birth, lat, lon, ay, a, com.kshana.app.data.Lang.code) },
                    firdaria = tlSection { engine.firdaria(birth, lat, lon, ay, com.kshana.app.data.Lang.code) },
                    fortune = tlSection { engine.zodiacalReleasing(birth, lat, lon, ay, "fortune", 2u, dayAfter, com.kshana.app.data.Lang.code) },
                    spirit = tlSection { engine.zodiacalReleasing(birth, lat, lon, ay, "spirit", 2u, dayAfter, com.kshana.app.data.Lang.code) },
                    progressed = tlSection { engine.progressions(birth, lat, lon, ay, a, com.kshana.app.data.Lang.code) },
                    hits = tlSection { engine.progressedHits(birth, lat, lon, ay, a, ExactInstant(a.taiSeconds + TL_TWO_YEARS_S, a.nanos), com.kshana.app.data.Lang.code) },
                    lunar = tlSection { engine.lunarReturns(birth, lat, lon, ay, a, ExactInstant(a.taiSeconds + TL_LUNAR_SPAN_S, a.nanos)) },
                )
            }
            cache[a] = r
        } catch (e: CancellationException) {
            throw e
        } catch (e: Exception) {
            loadError = e.message ?: e.toString()
        }
    }

    val load = cache[at]
    val loading = load == null && loadError == null

    Column(
        Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(horizontal = 24.dp).padding(top = 48.dp, bottom = 32.dp),
        verticalArrangement = Arrangement.spacedBy(10.dp),
    ) {
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween, verticalAlignment = Alignment.CenterVertically) {
            Text("TIME LORDS", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
            QuietButton("BACK", onClick = onBack)
        }
        Text("Time lords", style = MaterialTheme.typography.displayMedium, color = KshanaColors.Text)
        Text(
            "Every period system side by side at one instant · ${profile.name} · ${profile.placeName}",
            style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted,
        )

        // The instant: a day or a year either way, or now.
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(8.dp), verticalAlignment = Alignment.CenterVertically) {
            QuietButton("‹ DAY", onClick = { at = shiftAt(at, zone, -1, 0); isNow = false })
            QuietButton("DAY ›", onClick = { at = shiftAt(at, zone, 1, 0); isNow = false })
            Spacer(Modifier.weight(1f))
            Chip("NOW", selected = isNow, onClick = { at = Instants.now(); isNow = true })
        }
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(8.dp), verticalAlignment = Alignment.CenterVertically) {
            QuietButton("‹ YEAR", onClick = { at = shiftAt(at, zone, 0, -1); isNow = false })
            QuietButton("YEAR ›", onClick = { at = shiftAt(at, zone, 0, 1); isNow = false })
        }
        Row(verticalAlignment = Alignment.Bottom) {
            Text(localText(at, DATE_TIME_S, zone), fontFamily = KshanaFonts.mono, fontSize = 18.sp, color = KshanaColors.Sky)
            Text(microText(at), fontFamily = KshanaFonts.mono, fontSize = 12.sp, color = KshanaColors.Gold, modifier = Modifier.padding(start = 6.dp, bottom = 2.dp))
        }
        Text(
            if (loading) "Laying out every period system at this instant…" else zone.id,
            style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted,
        )
        ErrorLine(loadError)

        // ---- Vedic dashas ----
        SectionLabel("Vedic dashas")
        if (load == null) {
            if (loading) TlLoading("Running the five dashas from the birth Moon…")
        } else {
            ErrorLine(load.dashas.error)
            val systems = load.dashas.value
            if (systems != null) {
                if (systems.isEmpty()) TlLoading("The engine returned no dasha systems.")
                for (d in systems) {
                    DashaSystemCard(d, at, zone, notesOpen[d.system] == true) { notesOpen[d.system] = !(notesOpen[d.system] ?: false) }
                }
            }
        }

        // ---- Hellenistic and Western ----
        SectionLabel("Hellenistic and Western")
        if (load == null) {
            if (loading) TlLoading("Counting the profections, firdaria and releasings…")
        } else {
            TlCard {
                TlCardTitle("Profection", "ANNUAL")
                ErrorLine(load.profection.error)
                load.profection.value?.let { ProfectionBody(it, zone) }
            }
            TlCard {
                TlCardTitle("Firdaria", "75 YEARS")
                ErrorLine(load.firdaria.error)
                load.firdaria.value?.let { FirdariaBody(it, at, zone) }
            }
            TlCard {
                TlCardTitle("Zodiacal releasing", "L1 · L2")
                ErrorLine(load.fortune.error)
                load.fortune.value?.let { ZrLot("Lot of Fortune", it, at, zone) }
                HorizontalDivider(color = KshanaColors.LineSoft)
                ErrorLine(load.spirit.error)
                load.spirit.value?.let { ZrLot("Lot of Spirit", it, at, zone) }
            }
            TlCard {
                TlCardTitle("Progressions", "SECONDARY")
                ErrorLine(load.progressed.error)
                load.progressed.value?.let { ProgressedBody(it, zone) }
                HorizontalDivider(color = KshanaColors.LineSoft)
                Text("NEXT EXACT HITS · TWO YEARS", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted)
                ErrorLine(load.hits.error)
                load.hits.value?.let { HitsBody(it, at, zone) }
            }
            TlCard {
                TlCardTitle("Lunar returns", "NEXT THREE")
                ErrorLine(load.lunar.error)
                load.lunar.value?.let { LunarBody(it, at, zone) }
            }
        }

        Text(
            "Every boundary is exact arithmetic from the birth instant or a solved root; conventions are listed per system.",
            fontFamily = KshanaFonts.mono, fontSize = 10.sp, lineHeight = 15.sp, color = KshanaColors.Muted,
            modifier = Modifier.padding(vertical = 8.dp),
        )
    }
}

@Composable
private fun TlLoading(text: String) {
    Text(text, style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted)
}

/** A night card with a hairline. */
@Composable
private fun TlCard(content: @Composable () -> Unit) {
    val shape = RoundedCornerShape(8.dp)
    Column(
        Modifier.fillMaxWidth().clip(shape).background(KshanaColors.Surface).border(1.dp, KshanaColors.Line, shape).padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(6.dp),
    ) {
        content()
    }
}

@Composable
private fun TlCardTitle(title: String, tag: String) {
    Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
        Text(title, fontFamily = KshanaFonts.serif, fontSize = 20.sp, color = KshanaColors.Text, modifier = Modifier.weight(1f))
        Text(tag, style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted)
    }
}

/** A small bordered capital label. */
@Composable
private fun TlTag(text: String, colour: Color = KshanaColors.Gold) {
    Text(
        text,
        style = MaterialTheme.typography.labelSmall, color = colour,
        modifier = Modifier.border(1.dp, colour, RoundedCornerShape(4.dp)).padding(horizontal = 6.dp, vertical = 3.dp),
    )
}

@Composable
private fun TlBounds(start: ExactInstant, end: ExactInstant, zone: ZoneId, colour: Color = KshanaColors.Sky) {
    Text(
        "${localText(start, DATE_TIME_S, zone)} → ${localText(end, DATE_TIME_S, zone)}",
        fontFamily = KshanaFonts.mono, fontSize = 11.sp, lineHeight = 16.sp, color = colour,
    )
}

/**
 * A strip across [start]–[end]: each segment proportional to its length, the one holding
 * [at] in gold, the others in the line colour, a short label inside when it fits, the
 * instant as a gold line.
 */
@Composable
private fun TlStrip(start: ExactInstant, end: ExactInstant, segments: List<TlSeg>, at: ExactInstant) {
    val measurer = rememberTextMeasurer()
    val onStyle = TextStyle(fontFamily = KshanaFonts.mono, fontSize = 9.sp, color = KshanaColors.Night)
    val offStyle = TextStyle(fontFamily = KshanaFonts.mono, fontSize = 9.sp, color = KshanaColors.Muted)
    val s0 = start.seconds()
    val span = end.seconds() - s0
    val atS = at.seconds()
    Canvas(Modifier.fillMaxWidth().height(22.dp)) {
        if (span <= 0.0) return@Canvas
        fun x(s: Double): Float = (((s - s0) / span).coerceIn(0.0, 1.0) * size.width).toFloat()
        drawRect(KshanaColors.LineSoft)
        for (seg in segments) {
            val l = x(seg.start.seconds())
            val r = x(seg.end.seconds())
            val w = (r - l - 1f).coerceAtLeast(1f)
            val on = atS >= seg.start.seconds() && atS < seg.end.seconds()
            drawRect(if (on) KshanaColors.Gold else KshanaColors.Line, topLeft = Offset(l, 0f), size = Size(w, size.height))
            val lay = measurer.measure(tlShortLabel(seg.label), if (on) onStyle else offStyle)
            if (lay.size.width + 4f <= w) {
                drawText(lay, topLeft = Offset(l + (w - lay.size.width) / 2f, (size.height - lay.size.height) / 2f))
            }
        }
        if (atS >= s0 && atS <= s0 + span) {
            val xa = x(atS)
            drawLine(KshanaColors.Gold, Offset(xa, 0f), Offset(xa, size.height), 2f)
        }
    }
}

/** One dasha system: the running mahadasha, its sub-periods as a strip, the conventions on request. */
@Composable
private fun DashaSystemCard(d: DashaSystemRecord, at: ExactInstant, zone: ZoneId, notesOpen: Boolean, onToggleNotes: () -> Unit) {
    val l1 = d.running.firstOrNull { it.level.toInt() == 1 }
    val subs = d.running.filter { it.level.toInt() == 2 }
    val runningSub = subs.firstOrNull { tlWithin(it.start, it.end, at) }
    TlCard {
        Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
            Text(d.system.replaceFirstChar { it.uppercase() }, fontFamily = KshanaFonts.serif, fontSize = 20.sp, color = KshanaColors.Text, modifier = Modifier.weight(1f))
            if (!d.applies) {
                Text("does not apply here", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted)
            }
        }
        if (l1 == null) {
            Text("The instant falls outside the laid-out periods.", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted)
        } else {
            Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
                Text("${planetGlyphFor(l1.label)} ${l1.label}".trim(), fontFamily = KshanaFonts.serif, fontSize = 17.sp, color = KshanaColors.Gold, modifier = Modifier.weight(1f))
                Text("LEVEL 1", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted)
            }
            TlBounds(l1.start, l1.end, zone)
            if (subs.isNotEmpty()) {
                TlStrip(l1.start, l1.end, subs.map { TlSeg(it.label, it.start, it.end) }, at)
                if (runningSub != null) {
                    Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
                        Text(runningSub.label, style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Gold, modifier = Modifier.weight(1f))
                        Text("LEVEL 2", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted)
                    }
                    TlBounds(runningSub.start, runningSub.end, zone)
                }
            }
        }
        if (d.notes.isNotEmpty()) {
            Row(
                Modifier.fillMaxWidth().height(36.dp).clickable(onClick = onToggleNotes),
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.SpaceBetween,
            ) {
                Text("CONVENTIONS", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
                Text(if (notesOpen) "hide" else "show", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted)
            }
            if (notesOpen) {
                for (n in d.notes) {
                    Row(verticalAlignment = Alignment.Top) {
                        Text("•", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Gold, modifier = Modifier.width(16.dp))
                        Text(n, style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted, modifier = Modifier.weight(1f))
                    }
                }
            }
        }
    }
}

@Composable
private fun ProfectionBody(p: ProfectionRecord, zone: ZoneId) {
    Text("Year ${p.age.toInt()} · month ${p.month.toInt() + 1} of 12", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted)
    Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
        Text("${signGlyphFor(p.yearSign)} ${p.yearSign}".trim(), fontFamily = KshanaFonts.serif, fontSize = 20.sp, color = KshanaColors.Text, modifier = Modifier.weight(1f))
        Text("${planetGlyphFor(p.yearLord)} ${p.yearLord}".trim(), fontFamily = KshanaFonts.serif, fontSize = 20.sp, color = KshanaColors.Gold)
    }
    TlBounds(p.yearStart, p.yearEnd, zone)
    HorizontalDivider(color = KshanaColors.LineSoft)
    Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
        Text("${signGlyphFor(p.monthSign)} ${p.monthSign}".trim(), fontFamily = KshanaFonts.serif, fontSize = 17.sp, color = KshanaColors.Text, modifier = Modifier.weight(1f))
        Text("${planetGlyphFor(p.monthLord)} ${p.monthLord}".trim(), fontFamily = KshanaFonts.serif, fontSize = 17.sp, color = KshanaColors.Gold)
    }
    TlBounds(p.monthStart, p.monthEnd, zone)
    Text("the year's sign counted one per year from the Ascendant · the month one per month from the year's sign", style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted)
}

@Composable
private fun FirdariaBody(list: List<FirdariaRecord>, at: ExactInstant, zone: ZoneId) {
    val i = list.indexOfFirst { tlWithin(it.start, it.end, at) }
    if (i < 0) {
        Text("The instant falls outside the 75-year firdaria sequence.", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted)
        return
    }
    val cur = list[i]
    var j0 = i
    while (j0 > 0 && list[j0 - 1].lord == cur.lord) j0--
    var j1 = i
    while (j1 < list.lastIndex && list[j1 + 1].lord == cur.lord) j1++
    val majorStart = list[j0].start
    val majorEnd = list[j1].end
    Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
        Text("${planetGlyphFor(cur.lord)} ${cur.lord}".trim(), fontFamily = KshanaFonts.serif, fontSize = 20.sp, color = KshanaColors.Gold, modifier = Modifier.weight(1f))
        Text("MAJOR", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted)
    }
    TlBounds(majorStart, majorEnd, zone)
    val subs = (j0..j1).map { k -> TlSeg(list[k].sub ?: list[k].lord, list[k].start, list[k].end) }
    if (subs.size > 1) TlStrip(majorStart, majorEnd, subs, at)
    val sub = cur.sub
    if (sub != null) {
        Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
            Text("${planetGlyphFor(sub)} $sub".trim(), fontFamily = KshanaFonts.serif, fontSize = 17.sp, color = KshanaColors.Text, modifier = Modifier.weight(1f))
            Text("SUB", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted)
        }
        TlBounds(cur.start, cur.end, zone)
    } else {
        Text("an undivided node period", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted)
    }
}

/** The running L1 and L2 of one lot, peak and loosing of the bond tagged in gold. */
@Composable
private fun ZrLot(title: String, periods: List<ZrRecord>, at: ExactInstant, zone: ZoneId) {
    val l1 = periods.firstOrNull { it.level.toInt() == 1 && tlWithin(it.start, it.end, at) }
    val l2 = periods.firstOrNull { it.level.toInt() == 2 && tlWithin(it.start, it.end, at) }
    Text(title.uppercase(), style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted)
    if (l1 == null && l2 == null) {
        Text("No releasing period holds this instant.", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted)
        return
    }
    for ((level, p) in listOf(1 to l1, 2 to l2)) {
        if (p == null) continue
        Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            Text(
                "${signGlyphFor(p.sign)} ${p.sign}".trim(),
                fontFamily = KshanaFonts.serif, fontSize = if (level == 1) 20.sp else 17.sp,
                color = if (level == 1) KshanaColors.Gold else KshanaColors.Text, modifier = Modifier.weight(1f),
            )
            if (p.peak) TlTag("PEAK")
            if (p.loosing) TlTag("LOOSING OF THE BOND")
            Text("L$level", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted)
        }
        TlBounds(p.start, p.end, zone)
    }
}

@Composable
private fun ProgressedBody(pr: ProgressedRecord, zone: ZoneId) {
    val sun = pr.bodies.firstOrNull { it.key == "Sun" }
    val moon = pr.bodies.firstOrNull { it.key == "Moon" }
    MonoRow("Progressed Sun", sun?.let { tlLongitude(it.longitude) } ?: "—")
    MonoRow("Progressed Moon", moon?.let { tlLongitude(it.longitude) } ?: "—")
    MonoRow("Progressed Ascendant", tlLongitude(pr.asc))
    MonoRow("Progressed MC", tlLongitude(pr.mc))
    MonoRow("Solar arc", "%.4f°".format(pr.solarArc), KshanaColors.Gold)
    MonoRow("Lunation phase", "${pr.lunationPhase} · ${"%.1f".format(norm360(pr.moonPhaseDeg))}°")
    MonoRow("Ephemeris instant", localText(pr.instant, DATE_TIME, zone), KshanaColors.Muted)
    Text("a day after birth for each year lived: birth + elapsed / 365.2422", style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted)
}

@Composable
private fun HitsBody(hits: List<DirectionHitRecord>, at: ExactInstant, zone: ZoneId) {
    val atS = at.seconds()
    val next = hits.filter { it.at.seconds() >= atS }.sortedBy { it.at.seconds() }.take(5)
    if (next.isEmpty()) {
        Text("No exact hit in the next two years.", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted)
        return
    }
    for (h in next) {
        val kind = if (h.kind == "solar_arc") "SA" else "P"
        val retro = if (h.retrograde == true) " ℞" else ""
        Row(Modifier.fillMaxWidth().padding(vertical = 2.dp), verticalAlignment = Alignment.Top) {
            Text(kind, fontFamily = KshanaFonts.mono, fontSize = 10.sp, color = KshanaColors.Gold, modifier = Modifier.width(24.dp).padding(top = 2.dp))
            Text(
                "${planetGlyphFor(h.moving)} ${h.moving} ${h.aspect} ${h.natal}$retro".trim(),
                style = MaterialTheme.typography.bodyMedium, color = KshanaColors.TextSoft, modifier = Modifier.weight(1f),
            )
            Text(localText(h.at, DATE_TIME, zone), fontFamily = KshanaFonts.mono, fontSize = 10.sp, color = KshanaColors.Sky)
        }
    }
    Text("P progressed Sun or Moon · SA solar arc · ℞ the progressed body is retrograde", style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted)
}

@Composable
private fun LunarBody(returns: List<ExactInstant>, at: ExactInstant, zone: ZoneId) {
    val atS = at.seconds()
    val next = returns.filter { it.seconds() >= atS }.sortedBy { it.seconds() }.take(3)
    if (next.isEmpty()) {
        Text("No lunar return in the next hundred days.", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted)
        return
    }
    for ((i, r) in next.withIndex()) {
        val days = ((r.seconds() - atS) / TL_DAY_S.toDouble()).toInt()
        Row(Modifier.fillMaxWidth().padding(vertical = 2.dp), verticalAlignment = Alignment.CenterVertically) {
            Text("Return ${i + 1}", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted, modifier = Modifier.weight(1f))
            Text("in $days d", fontFamily = KshanaFonts.mono, fontSize = 10.sp, color = KshanaColors.Muted, modifier = Modifier.padding(end = 10.dp))
            Text(localText(r, DATE_TIME_S, zone), fontFamily = KshanaFonts.mono, fontSize = 12.sp, color = if (i == 0) KshanaColors.Gold else KshanaColors.Sky)
        }
    }
    Text("the Moon back at its natal sidereal longitude, solved as a root", style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted)
}
