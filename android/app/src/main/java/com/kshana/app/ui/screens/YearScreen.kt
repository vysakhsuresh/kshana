package com.kshana.app.ui.screens

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
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
import androidx.compose.runtime.mutableLongStateOf
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
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.drawText
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.rememberTextMeasurer
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.kshana.app.data.BirthProfile
import com.kshana.app.data.EngineSettings
import com.kshana.app.data.Place
import com.kshana.app.engine.EngineProvider
import com.kshana.app.engine.Instants
import com.kshana.app.ui.theme.KshanaColors
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import uniffi.kshana_ffi.ExactInstant
import uniffi.kshana_ffi.MuddaRecord
import uniffi.kshana_ffi.SahamRecord
import uniffi.kshana_ffi.TajikaYogaRecord
import uniffi.kshana_ffi.VarshaphalaRecord
import java.time.LocalDate
import java.time.ZoneId
import com.kshana.app.ui.sky.KshanaFonts

/** The saham groups, by the meaning's keywords. */
private enum class SahamGroup(val label: String) {
    ALL("All"), FORTUNE("Fortune"), LEARNING("Learning"), FAMILY("Family"), WORK("Work"), HEALTH("Health"), OTHER("Other"),
}

private fun sahamGroupOf(s: SahamRecord): SahamGroup {
    val t = "${s.name} ${s.meaning}".lowercase()
    fun any(vararg keys: String): Boolean = keys.any { t.contains(it) }
    return when {
        any("fortune", "merit", "fame") -> SahamGroup.FORTUNE
        any("learning") -> SahamGroup.LEARNING
        any("mother", "father", "child", "brother", "marriage", "spouse") -> SahamGroup.FAMILY
        any("work", "profession", "trade", "wealth") -> SahamGroup.WORK
        any("disease", "death", "danger") -> SahamGroup.HEALTH
        else -> SahamGroup.OTHER
    }
}

private fun ordinal(n: Int): String {
    val suffix = when {
        n % 100 in 11..13 -> "th"
        n % 10 == 1 -> "st"
        n % 10 == 2 -> "nd"
        n % 10 == 3 -> "rd"
        else -> "th"
    }
    return "$n$suffix"
}

private fun within(start: ExactInstant, end: ExactInstant, at: ExactInstant): Boolean {
    val s = at.seconds()
    return s >= start.seconds() && s < end.seconds()
}

/**
 * The calendar year whose solar return is the latest before today, from the birthday:
 * this year once the birthday has passed, else last year. The exact return refines it.
 */
private fun defaultReturnYear(profile: BirthProfile, zone: ZoneId): Int {
    val today = LocalDate.now(zone)
    val birthday = runCatching { LocalDate.of(today.year, profile.month, profile.day) }
        .getOrElse { LocalDate.of(today.year, profile.month, 28) }
    return if (today.isBefore(birthday)) today.year - 1 else today.year
}

/**
 * The year ahead: the Tajika varshaphala for the solar return of one calendar year, seen
 * from where the person is now. The annual chart, the lord of the year with the candidate
 * table, the sahams, the Tajika yogas and the Mudda dasha across the year.
 */
@Composable
fun YearScreen(profile: BirthProfile, here: Place, settings: EngineSettings, onBack: () -> Unit) {
    val context = LocalContext.current
    val zone = remember(here) { runCatching { ZoneId.of(here.zoneId) }.getOrDefault(ZoneId.systemDefault()) }
    val now = remember(profile, here, settings) { Instants.now() }
    val thisYear = remember(zone) { LocalDate.now(zone).year }
    val years = remember(thisYear) { ((thisYear - 1)..(thisYear + 2)).map { it.toLong() } }
    val defaultYear = remember(profile, zone) { defaultReturnYear(profile, zone).toLong() }
    var year by remember(profile, here, settings) { mutableLongStateOf(defaultYear) }
    var refined by remember(profile, here, settings) { mutableStateOf(false) }
    val cache = remember(profile, here, settings) { mutableStateMapOf<Long, VarshaphalaRecord>() }
    val errors = remember(profile, here, settings) { mutableStateMapOf<Long, String>() }
    var group by remember { mutableStateOf(SahamGroup.ALL) }
    var picked by remember(profile, here, settings) { mutableStateOf<Int?>(null) }
    var showBalas by remember { mutableStateOf(false) }
    var showAllMudda by remember { mutableStateOf(false) }

    // One return per year, solved off the main thread and kept.
    LaunchedEffect(profile, here, settings, year) {
        if (cache.containsKey(year)) return@LaunchedEffect
        errors.remove(year)
        try {
            val engine = EngineProvider.get(context)
            val birth = profile.birthInstant()
            val y = year
            val r = withContext(Dispatchers.Default) {
                engine.varshaphala(birth, profile.latitude, profile.longitude, settings.ayanamsa, y, here.latitude, here.longitude)
            }
            cache[y] = r
        } catch (e: CancellationException) {
            throw e
        } catch (e: Exception) {
            errors[year] = e.message ?: e.toString()
        }
    }

    val rec = cache[year]

    // The birthday guess corrected by the exact return: the chosen year is the latest return before now.
    LaunchedEffect(rec) {
        if (refined || rec == null || year != defaultYear) return@LaunchedEffect
        refined = true
        val nowS = now.seconds()
        if (rec.instant.seconds() > nowS && (year - 1) in years) {
            year -= 1
        } else if (rec.nextReturn.seconds() <= nowS && (year + 1) in years) {
            year += 1
        }
    }

    Column(
        Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(top = 48.dp, bottom = 32.dp),
        verticalArrangement = Arrangement.spacedBy(10.dp),
    ) {
        Column(Modifier.padding(horizontal = 24.dp), verticalArrangement = Arrangement.spacedBy(10.dp)) {
            Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween, verticalAlignment = Alignment.CenterVertically) {
                Text("THE YEAR AHEAD", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
                QuietButton("BACK", onClick = onBack)
            }
            Text(
                if (rec != null) "Your ${ordinal(rec.age.toInt() + 1)} year" else "The year ahead",
                style = MaterialTheme.typography.displayMedium, color = KshanaColors.Text,
            )
            Text(
                "${profile.name} · Tajika varshaphala · the return seen from ${here.name}",
                style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted,
            )
        }

        Row(
            Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()).padding(horizontal = 24.dp),
            horizontalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            for (y in years) {
                Chip(y.toString(), selected = y == year, onClick = {
                    if (y != year) {
                        year = y
                        picked = null
                    }
                })
            }
        }

        Column(Modifier.padding(horizontal = 24.dp), verticalArrangement = Arrangement.spacedBy(10.dp)) {
            ErrorLine(errors[year])
            if (rec == null) {
                if (errors[year] == null) {
                    Text("Solving the Sun's return of $year to the microsecond…", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted)
                }
            } else {
                ReturnHeader(rec, zone)
                Box(Modifier.fillMaxWidth(), contentAlignment = Alignment.Center) { AnnualChart(rec) }
                Text(
                    "lagna ${degMin(rec.lagnaSidereal)} ${signName(rec.lagnaSidereal)} · Muntha ${degMin(rec.munthaLongitude)} ${signName(rec.munthaLongitude)}, house ${rec.munthaHouse.toInt()}, lord ${rec.munthaLord}",
                    fontFamily = KshanaFonts.mono, fontSize = 11.sp, lineHeight = 16.sp, color = KshanaColors.Sky,
                )

                SectionLabel("Lord of the year")
                YearLordCard(rec, showBalas) { showBalas = !showBalas }

                SectionLabel("Sahams")
                Row(Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    for (g in SahamGroup.entries) {
                        Chip(g.label.uppercase(), selected = g == group, onClick = { group = g })
                    }
                }
                val sahams = if (group == SahamGroup.ALL) rec.sahams else rec.sahams.filter { sahamGroupOf(it) == group }
                SahamList(sahams)

                SectionLabel("Yogas")
                YogaList(rec.yogas)

                SectionLabel("Mudda dasha")
                val yearStart = rec.mudda.firstOrNull()?.start ?: rec.instant
                val yearEnd = rec.mudda.lastOrNull()?.end ?: rec.nextReturn
                MuddaBar(rec.mudda, yearStart, yearEnd, now, picked) { picked = if (picked == it) null else it }
                Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
                    Text(localText(yearStart, DATE_ONLY, zone), fontFamily = KshanaFonts.mono, fontSize = 10.sp, color = KshanaColors.Muted)
                    Text("${rec.mudda.size} periods across the year", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted)
                    Text(localText(yearEnd, DATE_ONLY, zone), fontFamily = KshanaFonts.mono, fontSize = 10.sp, color = KshanaColors.Muted)
                }
                MuddaDetail(rec.mudda, now, picked, zone)
                if (rec.mudda.isNotEmpty()) {
                    Row(
                        Modifier.fillMaxWidth().height(44.dp).clickable { showAllMudda = !showAllMudda },
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.SpaceBetween,
                    ) {
                        Text("ALL PERIODS", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
                        Text(if (showAllMudda) "hide" else "show", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted)
                    }
                    if (showAllMudda) {
                        for ((i, m) in rec.mudda.withIndex()) {
                            val running = within(m.start, m.end, now)
                            HorizontalDivider(color = KshanaColors.LineSoft)
                            Row(
                                Modifier.fillMaxWidth().height(44.dp).clickable { picked = i },
                                verticalAlignment = Alignment.CenterVertically,
                            ) {
                                Text(m.lord, fontFamily = KshanaFonts.serif, fontSize = 16.sp, color = if (running) KshanaColors.Gold else KshanaColors.Text, modifier = Modifier.weight(1f))
                                Text("${localText(m.start, DATE_TIME, zone)} → ${localText(m.end, DATE_TIME, zone)}", fontFamily = KshanaFonts.mono, fontSize = 10.sp, color = KshanaColors.Sky)
                            }
                        }
                        HorizontalDivider(color = KshanaColors.LineSoft)
                    }
                }

                Text(
                    "The return is the exact instant the Sun regains its natal sidereal longitude, solved to 1 µs · Tajika Neelakanthi, Charak",
                    fontFamily = KshanaFonts.mono, fontSize = 10.sp, lineHeight = 15.sp, color = KshanaColors.Muted,
                    modifier = Modifier.padding(vertical = 8.dp),
                )
            }
        }
    }
}

/** The return instant to the microsecond, day or night, and the next return. */
@Composable
private fun ReturnHeader(rec: VarshaphalaRecord, zone: ZoneId) {
    Column(verticalArrangement = Arrangement.spacedBy(3.dp)) {
        Text("THE RETURN", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted)
        Row(verticalAlignment = Alignment.Bottom) {
            Text(localText(rec.instant, DATE_TIME_S, zone), fontFamily = KshanaFonts.mono, fontSize = 18.sp, color = KshanaColors.Sky)
            Text(microText(rec.instant), fontFamily = KshanaFonts.mono, fontSize = 12.sp, color = KshanaColors.Gold, modifier = Modifier.padding(start = 6.dp, bottom = 2.dp))
        }
        Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(10.dp)) {
            YearTag(if (rec.day) "DAY" else "NIGHT", KshanaColors.Gold)
            Text("${zone.id} · age ${rec.age.toInt()} at the return", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted)
        }
        Text("next return ${localText(rec.nextReturn, DATE_TIME_S, zone)}", fontFamily = KshanaFonts.mono, fontSize = 11.sp, color = KshanaColors.Muted)
    }
}

/** A small bordered capital label. */
@Composable
private fun YearTag(text: String, colour: Color) {
    Text(
        text,
        style = MaterialTheme.typography.labelSmall, color = colour,
        modifier = Modifier.border(1.dp, colour, RoundedCornerShape(4.dp)).padding(horizontal = 6.dp, vertical = 3.dp),
    )
}

/**
 * The annual chart as a North Indian square: house 1 the top diamond, houses running
 * counter-clockwise, signs following the return lagna; the bodies as glyphs in the house
 * of their sign; the lagna "La" and the Muntha "Mu" in gold.
 */
@Composable
private fun AnnualChart(rec: VarshaphalaRecord) {
    val measurer = rememberTextMeasurer()
    val numStyle = TextStyle(fontFamily = KshanaFonts.mono, fontSize = 9.sp, color = KshanaColors.Muted)
    val planetStyle = TextStyle(fontFamily = KshanaFonts.sans, fontSize = 13.sp, color = KshanaColors.Text)
    val goldStyle = TextStyle(fontFamily = KshanaFonts.sans, fontSize = 12.sp, color = KshanaColors.Gold)
    val lagnaSign = signIndex(rec.lagnaSidereal)
    fun houseFor(lon: Double): Int = ((signIndex(lon) - lagnaSign + 12) % 12) + 1
    val byHouse: Map<Int, List<String>> = rec.bodies.groupBy { houseFor(it.sidereal) }
        .mapValues { (_, v) -> v.map { PLANET_GLYPHS[it.name] ?: it.name.take(2) } }
    val munthaHouse = houseFor(rec.munthaLongitude)
    Canvas(Modifier.size(320.dp)) {
        val s = size.minDimension
        val o = Offset((size.width - s) / 2f, (size.height - s) / 2f)
        fun p(x: Double, y: Double) = Offset(o.x + (x * s).toFloat(), o.y + (y * s).toFloat())
        val line = KshanaColors.Gold.copy(alpha = 0.55f)
        drawRect(KshanaColors.Surface, topLeft = o, size = Size(s, s))
        drawRect(line, topLeft = o, size = Size(s, s), style = Stroke(1.5f))
        drawLine(line, p(0.0, 0.0), p(1.0, 1.0), 1f)
        drawLine(line, p(1.0, 0.0), p(0.0, 1.0), 1f)
        drawLine(line, p(0.5, 0.0), p(1.0, 0.5), 1f)
        drawLine(line, p(1.0, 0.5), p(0.5, 1.0), 1f)
        drawLine(line, p(0.5, 1.0), p(0.0, 0.5), 1f)
        drawLine(line, p(0.0, 0.5), p(0.5, 0.0), 1f)
        // centroids of the twelve cells, house 1 at the top, counter-clockwise
        val centres = listOf(
            0.5 to 0.25, 0.25 to 1.0 / 12, 1.0 / 12 to 0.25, 0.25 to 0.5, 1.0 / 12 to 0.75, 0.25 to 11.0 / 12,
            0.5 to 0.75, 0.75 to 11.0 / 12, 11.0 / 12 to 0.75, 0.75 to 0.5, 11.0 / 12 to 0.25, 0.75 to 1.0 / 12,
        )
        // sign-number anchors, nudged toward each cell's apex so the bodies keep the centre
        val numbers = listOf(
            0.5 to 0.43, 0.25 to 0.03, 0.03 to 0.25, 0.43 to 0.5, 0.03 to 0.75, 0.25 to 0.97,
            0.5 to 0.57, 0.75 to 0.97, 0.97 to 0.75, 0.57 to 0.5, 0.97 to 0.25, 0.75 to 0.03,
        )
        for (h in 1..12) {
            val sign = (lagnaSign + h - 1) % 12 + 1
            val nl = measurer.measure(sign.toString(), numStyle)
            val np = p(numbers[h - 1].first, numbers[h - 1].second)
            drawText(nl, topLeft = Offset(np.x - nl.size.width / 2f, np.y - nl.size.height / 2f))
            val marks = listOfNotNull(if (h == 1) "La" else null, if (h == munthaHouse) "Mu" else null)
            val planetRows = (byHouse[h] ?: emptyList()).chunked(3)
            val rows: List<Pair<String, TextStyle>> =
                (if (marks.isEmpty()) emptyList() else listOf(marks.joinToString(" ") to goldStyle)) +
                    planetRows.map { it.joinToString(" ") to planetStyle }
            if (rows.isEmpty()) continue
            val cp = p(centres[h - 1].first, centres[h - 1].second)
            val lineH = 15.sp.toPx()
            val top = cp.y - (rows.size - 1) * lineH / 2f
            for ((ri, row) in rows.withIndex()) {
                val tl = measurer.measure(row.first, row.second)
                drawText(tl, topLeft = Offset(cp.x - tl.size.width / 2f, top + ri * lineH - tl.size.height / 2f))
            }
        }
    }
}

/** The varshesha, the candidate table with the winner in gold, the reasoning, and the balas on request. */
@Composable
private fun YearLordCard(rec: VarshaphalaRecord, showBalas: Boolean, onToggleBalas: () -> Unit) {
    val shape = RoundedCornerShape(8.dp)
    Column(
        Modifier.fillMaxWidth().clip(shape).background(KshanaColors.Surface).border(1.dp, KshanaColors.Line, shape).padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(8.dp),
    ) {
        Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(10.dp)) {
            Text(PLANET_GLYPHS[rec.varshesha] ?: "", fontSize = 26.sp, color = KshanaColors.Gold)
            Text(rec.varshesha, fontFamily = KshanaFonts.serif, fontSize = 28.sp, color = KshanaColors.Gold, modifier = Modifier.weight(1f))
            Text("VARSHESHA", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted)
        }
        HorizontalDivider(color = KshanaColors.LineSoft)
        Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
            Text("LORD", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted, modifier = Modifier.weight(1.1f))
            Text("OFFICES", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted, modifier = Modifier.weight(2f))
            Text("VISHWA", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted, textAlign = TextAlign.End, modifier = Modifier.width(52.dp))
            Text("LAGNA", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted, textAlign = TextAlign.End, modifier = Modifier.width(44.dp))
        }
        for (c in rec.candidates) {
            val win = c.lord == rec.varshesha
            Row(Modifier.fillMaxWidth().padding(vertical = 2.dp), verticalAlignment = Alignment.Top) {
                Text(c.lord, fontFamily = KshanaFonts.serif, fontSize = 15.sp, color = if (win) KshanaColors.Gold else KshanaColors.Text, modifier = Modifier.weight(1.1f))
                Text(
                    c.offices.joinToString(", ").ifEmpty { "—" },
                    style = MaterialTheme.typography.bodyMedium, color = if (win) KshanaColors.TextSoft else KshanaColors.Muted,
                    modifier = Modifier.weight(2f).padding(end = 6.dp),
                )
                Text("%.2f".format(c.vishwa), fontFamily = KshanaFonts.mono, fontSize = 12.sp, color = if (win) KshanaColors.Gold else KshanaColors.Sky, textAlign = TextAlign.End, modifier = Modifier.width(52.dp))
                Text(if (c.aspectsLagna) "✓" else "·", fontSize = 13.sp, color = if (c.aspectsLagna) KshanaColors.Gold else KshanaColors.Muted, textAlign = TextAlign.End, modifier = Modifier.width(44.dp))
            }
        }
        if (rec.reasoning.isNotEmpty()) {
            HorizontalDivider(color = KshanaColors.LineSoft)
            for (line in rec.reasoning) {
                Row(verticalAlignment = Alignment.Top) {
                    Text("•", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Gold, modifier = Modifier.width(16.dp))
                    Text(line, style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted, modifier = Modifier.weight(1f))
                }
            }
        }
        if (rec.balas.isNotEmpty()) {
            Row(
                Modifier.fillMaxWidth().height(36.dp).clickable(onClick = onToggleBalas),
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.SpaceBetween,
            ) {
                Text("PANCHAVARGEEYA BALA", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
                Text(if (showBalas) "hide" else "show", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted)
            }
            if (showBalas) {
                Row(Modifier.fillMaxWidth()) {
                    Text("", modifier = Modifier.weight(1.3f))
                    for (h in listOf("KSH", "UCH", "HAD", "DRE", "NAV", "TOT", "VIS")) {
                        Text(h, fontFamily = KshanaFonts.mono, fontSize = 9.sp, color = KshanaColors.Muted, textAlign = TextAlign.End, modifier = Modifier.weight(1f))
                    }
                }
                for (b in rec.balas) {
                    val win = b.planet == rec.varshesha
                    Row(Modifier.fillMaxWidth().padding(vertical = 1.dp)) {
                        Text(PLANET_ABBR[b.planet] ?: b.planet.take(2), fontFamily = KshanaFonts.sans, fontSize = 11.sp, color = if (win) KshanaColors.Gold else KshanaColors.TextSoft, modifier = Modifier.weight(1.3f))
                        for (v in listOf(b.kshetra, b.uchcha, b.hadda, b.drekkana, b.navamsa, b.total)) {
                            Text("%.1f".format(v), fontFamily = KshanaFonts.mono, fontSize = 10.sp, color = KshanaColors.Sky, textAlign = TextAlign.End, modifier = Modifier.weight(1f))
                        }
                        Text("%.2f".format(b.vishwa), fontFamily = KshanaFonts.mono, fontSize = 10.sp, color = if (win) KshanaColors.Gold else KshanaColors.Sky, textAlign = TextAlign.End, modifier = Modifier.weight(1f))
                    }
                }
                Text("kshetra · uchcha · hadda · drekkana · navamsa · total · vishwa bala", fontFamily = KshanaFonts.mono, fontSize = 9.sp, color = KshanaColors.Muted)
            }
        }
    }
}

/** Two columns of sahams: name, meaning, sign and degree, house. */
@Composable
private fun SahamList(sahams: List<SahamRecord>) {
    if (sahams.isEmpty()) {
        Text("No sahams in this group.", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted)
        return
    }
    for (pair in sahams.chunked(2)) {
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(12.dp)) {
            for (s in pair) SahamCell(s, Modifier.weight(1f))
            if (pair.size == 1) Spacer(Modifier.weight(1f))
        }
    }
}

@Composable
private fun SahamCell(s: SahamRecord, modifier: Modifier = Modifier) {
    Column(modifier.padding(vertical = 4.dp), verticalArrangement = Arrangement.spacedBy(1.dp)) {
        Text(s.name, fontFamily = KshanaFonts.serif, fontSize = 16.sp, color = KshanaColors.Text)
        Text(s.meaning, style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted, maxLines = 2, overflow = TextOverflow.Ellipsis)
        Text(
            "${SIGN_SHORT[signIndex(s.longitude)]} ${degMin(s.longitude)} · H${s.house.toInt()}",
            fontFamily = KshanaFonts.mono, fontSize = 11.sp, color = KshanaColors.Sky,
        )
    }
}

/** The Tajika yogas, the exact ones in gold. */
@Composable
private fun YogaList(yogas: List<TajikaYogaRecord>) {
    if (yogas.isEmpty()) {
        Text("No Tajika yoga forms in this return.", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted)
        return
    }
    for (y in yogas) {
        HorizontalDivider(color = KshanaColors.LineSoft)
        Column(Modifier.fillMaxWidth().padding(vertical = 6.dp), verticalArrangement = Arrangement.spacedBy(2.dp)) {
            Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                Text(y.name, fontFamily = KshanaFonts.serif, fontSize = 17.sp, color = if (y.exact) KshanaColors.Gold else KshanaColors.Text, modifier = Modifier.weight(1f))
                if (y.exact) YearTag("EXACT", KshanaColors.Gold)
            }
            Text(
                "${y.kind} · ${y.planets.joinToString(" ") { "${PLANET_GLYPHS[it] ?: ""} $it".trim() }}",
                fontFamily = KshanaFonts.mono, fontSize = 10.sp, color = KshanaColors.Muted,
            )
            Text(y.words, style = MaterialTheme.typography.bodyMedium, color = if (y.exact) KshanaColors.Text else KshanaColors.TextSoft)
        }
    }
    HorizontalDivider(color = KshanaColors.LineSoft)
}

/**
 * The Mudda dasha across the year: one bar per period, coloured gold, sky and text in turn
 * at 60 %, the lord's abbreviation inside when it fits, the picked period outlined, now in gold.
 */
@Composable
private fun MuddaBar(
    mudda: List<MuddaRecord>,
    yearStart: ExactInstant,
    yearEnd: ExactInstant,
    now: ExactInstant,
    picked: Int?,
    onPick: (Int) -> Unit,
) {
    val measurer = rememberTextMeasurer()
    val labelStyle = TextStyle(fontFamily = KshanaFonts.mono, fontSize = 9.sp, color = KshanaColors.Night)
    val palette = listOf(KshanaColors.Gold, KshanaColors.Sky, KshanaColors.Text).map { it.copy(alpha = 0.6f) }
    val s0 = yearStart.seconds()
    val span = yearEnd.seconds() - s0
    val nowS = now.seconds()
    Canvas(
        Modifier.fillMaxWidth().height(44.dp).pointerInput(mudda, s0, span) {
            detectTapGestures { off ->
                if (span <= 0.0 || size.width <= 0) return@detectTapGestures
                val t = s0 + (off.x / size.width).toDouble() * span
                val i = mudda.indexOfFirst { it.start.seconds() <= t && t < it.end.seconds() }
                if (i >= 0) onPick(i)
            }
        },
    ) {
        if (span <= 0.0) return@Canvas
        val barTop = 8.dp.toPx()
        val barH = size.height - barTop - 6.dp.toPx()
        fun x(s: Double): Float = (((s - s0) / span).coerceIn(0.0, 1.0) * size.width).toFloat()
        drawRect(KshanaColors.LineSoft, topLeft = Offset(0f, barTop), size = Size(size.width, barH))
        for ((i, m) in mudda.withIndex()) {
            val l = x(m.start.seconds())
            val r = x(m.end.seconds())
            val w = (r - l - 1f).coerceAtLeast(1f)
            drawRect(palette[i % palette.size], topLeft = Offset(l, barTop), size = Size(w, barH))
            if (picked == i) {
                drawRect(KshanaColors.Text, topLeft = Offset(l, barTop), size = Size(w, barH), style = Stroke(1.5f))
            }
            val lay = measurer.measure(PLANET_ABBR[m.lord] ?: m.lord.take(2), labelStyle)
            if (lay.size.width + 4f <= w) {
                drawText(lay, topLeft = Offset(l + (w - lay.size.width) / 2f, barTop + (barH - lay.size.height) / 2f))
            }
        }
        if (nowS >= s0 && nowS <= s0 + span) {
            val xn = x(nowS)
            drawLine(KshanaColors.Gold, Offset(xn, barTop - 2f), Offset(xn, size.height), 2f)
            val tri = Path().apply {
                moveTo(xn - 5.dp.toPx(), 0f)
                lineTo(xn + 5.dp.toPx(), 0f)
                lineTo(xn, barTop - 1f)
                close()
            }
            drawPath(tri, KshanaColors.Gold)
        }
    }
}

/** The running (or picked) Mudda lord with its antardashas and exact local bounds. */
@Composable
private fun MuddaDetail(mudda: List<MuddaRecord>, now: ExactInstant, picked: Int?, zone: ZoneId) {
    if (mudda.isEmpty()) {
        Text("The engine returned no Mudda periods.", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted)
        return
    }
    val runningIdx = mudda.indexOfFirst { within(it.start, it.end, now) }
    val idx = picked ?: runningIdx
    if (idx < 0) {
        Text(
            if (now.seconds() < mudda.first().start.seconds()) "This year has not begun; tap a period to read it." else "This year has passed; tap a period to read it.",
            style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted,
        )
        return
    }
    val m = mudda[idx]
    val running = idx == runningIdx
    Column(Modifier.fillMaxWidth(), verticalArrangement = Arrangement.spacedBy(4.dp)) {
        Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            Text("${PLANET_GLYPHS[m.lord] ?: ""} ${m.lord} mudda".trim(), fontFamily = KshanaFonts.serif, fontSize = 20.sp, color = if (running) KshanaColors.Gold else KshanaColors.Text, modifier = Modifier.weight(1f))
            YearTag(if (running) "RUNNING" else "PICKED", if (running) KshanaColors.Gold else KshanaColors.Muted)
        }
        Text(
            "${localText(m.start, DATE_TIME_S, zone)} → ${localText(m.end, DATE_TIME_S, zone)}",
            fontFamily = KshanaFonts.mono, fontSize = 11.sp, color = KshanaColors.Sky,
        )
        val n = minOf(m.antarLords.size, m.antarStarts.size, m.antarEnds.size)
        for (i in 0 until n) {
            val on = running && within(m.antarStarts[i], m.antarEnds[i], now)
            Row(Modifier.fillMaxWidth().padding(start = 12.dp, top = 2.dp), verticalAlignment = Alignment.CenterVertically) {
                Text("${m.lord}–${m.antarLords[i]}", style = MaterialTheme.typography.bodyMedium, color = if (on) KshanaColors.Gold else KshanaColors.TextSoft, modifier = Modifier.weight(1f))
                Text(
                    "${localText(m.antarStarts[i], if (on) DATE_TIME_S else DATE_TIME, zone)} → ${localText(m.antarEnds[i], if (on) DATE_TIME_S else DATE_TIME, zone)}",
                    fontFamily = KshanaFonts.mono, fontSize = 10.sp, color = if (on) KshanaColors.Sky else KshanaColors.Muted,
                )
            }
            if (on) {
                Row(Modifier.padding(start = 12.dp)) {
                    Text("antardasha changes ${localText(m.antarEnds[i], DATE_TIME_S, zone)}", fontFamily = KshanaFonts.mono, fontSize = 10.sp, color = KshanaColors.Muted)
                    Text(microText(m.antarEnds[i]), fontFamily = KshanaFonts.mono, fontSize = 10.sp, color = KshanaColors.Gold, modifier = Modifier.padding(start = 4.dp))
                }
            }
        }
    }
}
