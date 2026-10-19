package com.kshana.app.ui.screens

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
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
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontStyle
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.kshana.app.data.BirthProfile
import com.kshana.app.data.Commitment
import com.kshana.app.data.CommitmentStore
import com.kshana.app.data.EngineSettings
import com.kshana.app.data.Metrics
import com.kshana.app.engine.EngineProvider
import com.kshana.app.engine.Instants
import com.kshana.app.ui.theme.KshanaColors
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import uniffi.kshana_ffi.DoshaRecord
import uniffi.kshana_ffi.PariharaRecord
import uniffi.kshana_ffi.RemedyRecord
import uniffi.kshana_ffi.RemedyWindowRecord
import java.time.Instant
import java.time.ZoneId
import java.time.format.DateTimeFormatter
import com.kshana.app.ui.sky.KshanaFonts

/** The eight life areas, in the engine's order; "All" asks for the running lords alone. */
private val AREAS = listOf("Career", "Marriage", "Children", "Wealth", "Health", "Property", "Education", "Foreign")
private const val ALL = "All"

private val DAY_WORDS = DateTimeFormatter.ofPattern("EEEE d MMM")
private val WHEN_WORDS = DateTimeFormatter.ofPattern("EEEE d MMM HH:mm:ss")

/** The ladder's rungs, as the engine numbers them. */
private fun rungLabel(rung: Int): String = when (rung) {
    1 -> "Karma"
    2 -> "Seva"
    3 -> "Vrata and discipline"
    4 -> "Daana"
    5 -> "Mantra"
    6 -> "Pooja"
    7 -> "Gem, informational"
    else -> "Rung $rung"
}

private fun kindColour(kind: String): Color = when (kind) {
    "Karma", "Seva", "Discipline", "Vrata" -> KshanaColors.Gold
    "Daana" -> KshanaColors.Sky
    else -> KshanaColors.Muted
}

private fun dayWords(millis: Long, zone: ZoneId): String = Instant.ofEpochMilli(millis).atZone(zone).format(DAY_WORDS)
private fun whenWords(millis: Long, zone: ZoneId): String = Instant.ofEpochMilli(millis).atZone(zone).format(WHEN_WORDS)

/**
 * "Earn the period.": the cause the engine locates, its karma class and what a remedy
 * may claim, the doshas the chart carries, the remedies ranked by rung with their next
 * exact windows, and the parihara ledger of what you planned and what you did.
 */
@Composable
fun PariharaScreen(profile: BirthProfile, settings: EngineSettings, onBack: () -> Unit) {
    val context = LocalContext.current
    val zone = remember { ZoneId.systemDefault() }
    var area by remember(profile) { mutableStateOf(ALL) }
    val cache = remember(profile, settings) { mutableStateMapOf<String, PariharaRecord>() }
    var loading by remember { mutableStateOf(false) }
    var error by remember { mutableStateOf<String?>(null) }
    var commitments by remember(profile) { mutableStateOf(CommitmentStore.commitments(context, profile.name)) }

    LaunchedEffect(profile, settings, area) {
        if (cache.containsKey(area)) return@LaunchedEffect
        loading = true
        try {
            val engine = EngineProvider.get(context)
            val birth = profile.birthInstant()
            val rec = withContext(Dispatchers.Default) {
                engine.parihara(birth, profile.latitude, profile.longitude, settings.ayanamsa, Instants.now(), if (area == ALL) null else area)
            }
            cache[area] = rec
            commitments = CommitmentStore.refreshPlans(context, profile.name, rec.remedies)
            error = null
        } catch (e: CancellationException) {
            throw e
        } catch (e: Exception) {
            error = e.message ?: e.toString()
        }
        loading = false
    }

    val rec = cache[area]
    val nowMillis = System.currentTimeMillis()
    val metrics = remember(commitments, nowMillis / 60_000L) { CommitmentStore.metrics(commitments, nowMillis) }

    Column(
        Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(top = 48.dp, bottom = 32.dp),
        verticalArrangement = Arrangement.spacedBy(10.dp),
    ) {
        Column(Modifier.padding(horizontal = 24.dp), verticalArrangement = Arrangement.spacedBy(10.dp)) {
            Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween, verticalAlignment = Alignment.CenterVertically) {
                Text("PARIHARA", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
                QuietButton("BACK", onClick = onBack)
            }
            Text("Earn the period.", style = MaterialTheme.typography.displayLarge, color = KshanaColors.Text)
            Text(
                "The cause the running lords locate, what the texts let a remedy claim, and the ledger of what you did · ${profile.placeName}",
                style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted,
            )
            ErrorLine(error)
        }

        Row(Modifier.horizontalScroll(rememberScrollState()).padding(horizontal = 24.dp), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            for (a in listOf(ALL) + AREAS) Chip(a.uppercase(), selected = a == area, onClick = { area = a })
        }

        Column(Modifier.padding(horizontal = 24.dp), verticalArrangement = Arrangement.spacedBy(10.dp)) {
            SectionLabel("The ledger")
            LedgerCard(metrics, commitments, rec, nowMillis, zone)

            if (rec == null) {
                if (loading || error == null) {
                    Text(
                        if (area == ALL) "weighing the running lords…" else "weighing the running lords and the lord of ${area.lowercase()}…",
                        style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted,
                    )
                }
            } else {
                SectionLabel("The cause")
                CauseCard(rec, zone)

                SectionLabel("Doshas present")
                val present = rec.doshas.filter { it.present }
                if (present.isEmpty()) {
                    Text("No curse yoga or dosha stands in this chart.", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted)
                } else {
                    for (d in present) DoshaRow(d)
                }

                SectionLabel("Remedies, ranked")
                if (rec.remedies.isEmpty()) {
                    Text("The engine names no remedy for this period.", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted)
                }
                var lastRung = -1
                for (r in rec.remedies) {
                    val rung = r.rung.toInt()
                    if (rung != lastRung) {
                        lastRung = rung
                        Text(
                            "$rung · ${rungLabel(rung)}".uppercase(),
                            style = MaterialTheme.typography.labelSmall,
                            color = if (rung == 7) KshanaColors.Muted else KshanaColors.Gold,
                            modifier = Modifier.padding(top = 8.dp),
                        )
                    }
                    RemedyCard(
                        r = r,
                        held = commitments.firstOrNull { it.ruleId == r.ruleId },
                        zone = zone,
                        nowMillis = nowMillis,
                        onHold = { commitments = CommitmentStore.commit(context, profile.name, r) },
                        onDone = { commitments = CommitmentStore.markDone(context, profile.name, r.ruleId) },
                        onRelease = { commitments = CommitmentStore.release(context, profile.name, r.ruleId) },
                    )
                }

                Text(rec.ledgerHint, style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted, modifier = Modifier.padding(top = 8.dp))
            }
            Text(
                "Every window is an exact instant from the solved sunrise; every remedy names its source. Times in ${zone.id}.",
                fontFamily = KshanaFonts.mono, fontSize = 10.sp, lineHeight = 15.sp, color = KshanaColors.Muted,
                modifier = Modifier.padding(vertical = 8.dp),
            )
        }
    }
}

/* ---------------------------------------------------------------------------------- */

/** The night card every block sits in. */
@Composable
private fun NightCard(dim: Boolean = false, content: @Composable () -> Unit) {
    val shape = RoundedCornerShape(8.dp)
    Column(
        Modifier.fillMaxWidth().clip(shape).background(KshanaColors.Surface).border(1.dp, if (dim) KshanaColors.LineSoft else KshanaColors.Line, shape).padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(8.dp),
    ) { content() }
}

/** A 3 dp bar: the track in LineSoft, the fraction in [colour]. */
@Composable
private fun ThinBar(fraction: Float, colour: Color, modifier: Modifier = Modifier) {
    Box(modifier.fillMaxWidth().height(3.dp).clip(RoundedCornerShape(2.dp)).background(KshanaColors.LineSoft)) {
        val f = fraction.coerceIn(0f, 1f)
        if (f > 0f) Box(Modifier.fillMaxWidth(f).height(3.dp).background(colour))
    }
}

/** "WHY" / "HIDE": a quiet toggle in the label style. */
@Composable
private fun ToggleLabel(open: Boolean, label: String, onToggle: () -> Unit) {
    Text(
        if (open) "HIDE" else label.uppercase(),
        style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold,
        modifier = Modifier.clickable(onClick = onToggle).padding(vertical = 8.dp),
    )
}

/* ---------------------------------------------------------------------------------- */

/** Kept over planned for 7 and 40 days, the lapse and its repair, and the clock the logs come from. */
@Composable
private fun LedgerCard(m: Metrics, commitments: List<Commitment>, rec: PariharaRecord?, nowMillis: Long, zone: ZoneId) {
    NightCard {
        if (commitments.isEmpty()) {
            Text("You hold no remedy yet.", fontFamily = KshanaFonts.serif, fontSize = 18.sp, lineHeight = 24.sp, color = KshanaColors.Text)
            Text("Hold one below and the ledger opens at its first window.", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted)
            return@NightCard
        }
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(16.dp)) {
            LedgerCount("7 DAYS", m.kept7, m.planned7, Modifier.weight(1f))
            LedgerCount("40 DAYS", m.kept40, m.planned40, Modifier.weight(1f))
        }
        Text("kept over planned · a log counts as kept; no streak is counted", fontFamily = KshanaFonts.mono, fontSize = 10.sp, color = KshanaColors.Muted)
        HorizontalDivider(color = KshanaColors.LineSoft)

        // The next window: the ledger's own plan first, else the engine's next window for a held remedy.
        val next = CommitmentStore.nextPlanned(commitments, nowMillis)
            ?: rec?.remedies
                ?.filter { r -> commitments.any { it.ruleId == r.ruleId } }
                ?.flatMap { it.windows }
                ?.map { CommitmentStore.millisOf(it.start) }
                ?.filter { it > nowMillis }
                ?.minOrNull()
        val nextWords = next?.let { "The next window is ${whenWords(it, zone)}." }
            ?: "No further window is planned; open this screen after the next sunrise and the plan extends."
        val lapse = m.lapses.lastOrNull()
        Text(
            when {
                lapse != null -> "You missed ${dayWords(lapse, zone)}. $nextWords Repair it there; one missed day does not void the vow."
                m.planned40 == 0 -> "Nothing has come due yet. $nextWords"
                else -> "You missed nothing in the last 40 days. $nextWords"
            },
            fontFamily = KshanaFonts.serif, fontSize = 17.sp, lineHeight = 24.sp, color = KshanaColors.Text,
        )
        Text(
            "${commitments.size} held · logs from the ${CommitmentStore.DEVICE_CLOCK} · windows from the engine",
            fontFamily = KshanaFonts.mono, fontSize = 10.sp, lineHeight = 15.sp, color = KshanaColors.Muted,
        )
    }
}

@Composable
private fun LedgerCount(label: String, kept: Int, planned: Int, modifier: Modifier = Modifier) {
    Column(modifier, verticalArrangement = Arrangement.spacedBy(4.dp)) {
        Text(label, style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted)
        Row(verticalAlignment = Alignment.Bottom) {
            Text(kept.toString(), fontFamily = KshanaFonts.mono, fontSize = 28.sp, color = KshanaColors.Text)
            Text(" / $planned", fontFamily = KshanaFonts.mono, fontSize = 14.sp, color = KshanaColors.Sky, modifier = Modifier.padding(bottom = 4.dp))
        }
        ThinBar(if (planned == 0) 0f else kept.toFloat() / planned.toFloat(), KshanaColors.Gold)
    }
}

/* ---------------------------------------------------------------------------------- */

/** The planet, the cause in words, where it stands, and the karma verdict with its basis. */
@Composable
private fun CauseCard(rec: PariharaRecord, zone: ZoneId) {
    var why by remember(rec) { mutableStateOf(false) }
    NightCard {
        Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(12.dp)) {
            Text(PLANET_GLYPHS[rec.planet] ?: rec.planet.take(2), fontSize = 28.sp, color = KshanaColors.Gold)
            Column {
                Text(rec.planet, style = MaterialTheme.typography.titleLarge, color = KshanaColors.Text)
                Text("severity ${"%.2f".format(rec.severity)}", fontFamily = KshanaFonts.mono, fontSize = 11.sp, color = KshanaColors.Sky)
            }
        }
        Text(rec.cause, fontFamily = KshanaFonts.serif, fontSize = 17.sp, lineHeight = 24.sp, color = KshanaColors.Text)
        val running = rec.runningLevel?.let { level ->
            val since = rec.runningSince?.let { " since ${localText(it, DATE_ONLY, zone)}" } ?: ""
            val until = rec.runningUntil?.let { " until ${localText(it, DATE_ONLY, zone)}" } ?: ""
            " · running $level$since$until"
        } ?: ""
        Text(
            "house ${rec.causeHouse.toInt()} · placed in ${rec.placedIn.toInt()} · ${rec.dignity.lowercase()}$running",
            fontFamily = KshanaFonts.mono, fontSize = 11.sp, lineHeight = 16.sp, color = KshanaColors.Sky,
        )
        HorizontalDivider(color = KshanaColors.LineSoft)

        // The karma verdict: class and kind as a chip, then what a remedy may claim.
        val shape = RoundedCornerShape(16.dp)
        Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(10.dp)) {
            Box(
                Modifier.clip(shape).border(1.dp, KshanaColors.Gold, shape).padding(horizontal = 12.dp, vertical = 6.dp),
                contentAlignment = Alignment.Center,
            ) {
                Text(rec.karmaClass.replaceFirstChar { it.uppercase() }, style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
            }
            Text(rec.karmaKind.lowercase(), fontFamily = KshanaFonts.mono, fontSize = 11.sp, color = KshanaColors.Muted)
        }
        Text(sentence(rec.mayClaim), fontFamily = KshanaFonts.serif, fontSize = 17.sp, lineHeight = 24.sp, color = KshanaColors.Gold)
        if (rec.karmaBasis.isNotEmpty()) {
            ToggleLabel(why, "why") { why = !why }
            if (why) {
                for (b in rec.karmaBasis) {
                    Row(verticalAlignment = Alignment.Top) {
                        Text("•", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Gold, modifier = Modifier.width(16.dp))
                        Text(sentence(b), style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted, modifier = Modifier.weight(1f))
                    }
                }
            }
        }
        Text(rec.karmaSource, fontFamily = KshanaFonts.mono, fontSize = 10.sp, lineHeight = 15.sp, color = KshanaColors.Muted)
    }
}

/** One present dosha: name, its severity as a thin bar, the offended being, factors on request, the source. */
@Composable
private fun DoshaRow(d: DoshaRecord) {
    var open by remember(d.ruleId) { mutableStateOf(false) }
    NightCard {
        Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
            Text(d.name, fontFamily = KshanaFonts.serif, fontSize = 17.sp, color = KshanaColors.Text, modifier = Modifier.weight(1f))
            Text("${PLANET_GLYPHS[d.planet] ?: ""} ${"%.2f".format(d.severity)}", fontFamily = KshanaFonts.mono, fontSize = 11.sp, color = KshanaColors.Sky)
        }
        ThinBar(d.severity.toFloat(), if (d.severity >= 0.5) KshanaColors.Gold else KshanaColors.Sky)
        if (d.offended.isNotBlank()) {
            Text("Offended: ${d.offended}", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.TextSoft)
        }
        if (d.remedyFamily.isNotBlank()) {
            Text("remedy family · ${d.remedyFamily}", fontFamily = KshanaFonts.mono, fontSize = 10.sp, color = KshanaColors.Muted)
        }
        if (d.factors.isNotEmpty()) {
            ToggleLabel(open, "factors") { open = !open }
            if (open) {
                for (f in d.factors) {
                    Row(verticalAlignment = Alignment.Top) {
                        Text("•", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Gold, modifier = Modifier.width(16.dp))
                        Text(sentence(f), style = MaterialTheme.typography.bodyMedium, color = KshanaColors.TextSoft, modifier = Modifier.weight(1f))
                    }
                }
            }
        }
        Text("${d.source} · ${d.ruleId}", fontFamily = KshanaFonts.mono, fontSize = 10.sp, lineHeight = 15.sp, color = KshanaColors.Muted)
    }
}

/* ---------------------------------------------------------------------------------- */

/**
 * One remedy: kind, title, why, what it may claim, its cadence, the next three exact
 * windows each with a reminder, what the ledger records, and the hold / did it / release
 * control. Gems carry no control: information only.
 */
@Composable
private fun RemedyCard(
    r: RemedyRecord,
    held: Commitment?,
    zone: ZoneId,
    nowMillis: Long,
    onHold: () -> Unit,
    onDone: () -> Unit,
    onRelease: () -> Unit,
) {
    val gem = r.kind == "Gem"
    val colour = kindColour(r.kind)
    val textColour = if (gem) KshanaColors.Muted else if (r.optional) KshanaColors.TextSoft else KshanaColors.Text
    NightCard(dim = gem) {
        Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
            Text(r.kind.uppercase(), style = MaterialTheme.typography.labelSmall, color = colour)
            if (gem) {
                Text(" · INFORMATION ONLY", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted)
            } else if (r.optional) {
                Text(" · OPTIONAL, NEVER REQUIRED", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted)
            }
            Spacer(Modifier.weight(1f))
            Text("fit ${"%.2f".format(r.fit)} · ${r.effort}", fontFamily = KshanaFonts.mono, fontSize = 10.sp, color = KshanaColors.Muted)
        }
        Text(r.title, fontFamily = KshanaFonts.serif, fontSize = 18.sp, lineHeight = 24.sp, color = textColour)
        Text(r.why, style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted)
        Text(
            sentence(r.mayClaim),
            fontFamily = KshanaFonts.serif, fontSize = 14.sp, lineHeight = 20.sp, fontStyle = FontStyle.Italic,
            color = if (gem) KshanaColors.Muted else KshanaColors.Gold,
        )
        val dose = buildList {
            add(r.cadence)
            r.count?.let { add("count $it") }
            r.durationDays?.let { add("$it days") }
        }.joinToString(" · ")
        Text(dose, fontFamily = KshanaFonts.mono, fontSize = 11.sp, color = if (gem) KshanaColors.Muted else KshanaColors.Sky)

        if (!gem && r.windows.isNotEmpty()) {
            HorizontalDivider(color = KshanaColors.LineSoft)
            Text("NEXT WINDOWS", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted)
            for (w in r.windows.take(3)) WindowRow(r, w, zone)
        }

        Text(r.measurable, fontFamily = KshanaFonts.mono, fontSize = 10.sp, lineHeight = 15.sp, color = KshanaColors.Muted)
        Text("${r.source} · ${r.ruleId}", fontFamily = KshanaFonts.mono, fontSize = 10.sp, lineHeight = 15.sp, color = KshanaColors.Muted)

        if (gem) return@NightCard
        HorizontalDivider(color = KshanaColors.LineSoft)
        if (held == null) {
            QuietButton("HOLD THIS", onClick = onHold, modifier = Modifier.fillMaxWidth())
        } else {
            val last = held.doneInstants.lastOrNull()
            Text(
                "held since ${dayWords(held.startedAtMillis, zone)} · ${held.doneInstants.size} logged" +
                    (last?.let { " · last ${whenWords(it, zone)}" } ?: "") +
                    " · ${held.plannedInstants.count { it > nowMillis }} windows ahead",
                fontFamily = KshanaFonts.mono, fontSize = 10.sp, lineHeight = 15.sp, color = KshanaColors.Gold,
            )
            Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(8.dp), verticalAlignment = Alignment.CenterVertically) {
                GoldButton("DID IT NOW", onClick = onDone, modifier = Modifier.weight(1f))
                QuietButton("RELEASE", onClick = onRelease)
            }
        }
    }
}

/** One exact window: local start and end to the second, the words, and a reminder at the start. */
@Composable
private fun WindowRow(r: RemedyRecord, w: RemedyWindowRecord, zone: ZoneId) {
    val start = Instants.toLocal(w.start, zone)
    val end = Instants.toLocal(w.end, zone)
    val endText = if (start.toLocalDate() == end.toLocalDate()) end.format(TIME_ONLY) else end.format(DATE_TIME_S)
    val avoid = w.kind.contains("avoid")
    Column(Modifier.fillMaxWidth(), verticalArrangement = Arrangement.spacedBy(2.dp)) {
        Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
            Column(Modifier.weight(1f)) {
                Text(
                    "${start.format(DATE_TIME_S)} – $endText",
                    fontFamily = KshanaFonts.mono, fontSize = 11.sp, color = if (avoid) KshanaColors.Muted else KshanaColors.Sky,
                )
                Text(
                    "${w.kind}${if (w.horaIndex.toInt() > 0) " · hora ${w.horaIndex.toInt()}" else ""} · ${w.words}",
                    style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted,
                )
            }
            if (!avoid) {
                RemindButton(
                    key = "${r.ruleId}@${w.start.taiSeconds}",
                    atMillis = CommitmentStore.millisOf(w.start),
                    title = r.title,
                    text = w.words,
                    modifier = Modifier.padding(start = 8.dp),
                )
            }
        }
    }
}
