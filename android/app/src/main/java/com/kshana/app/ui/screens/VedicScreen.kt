package com.kshana.app.ui.screens

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
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
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.drawscope.Stroke
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
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import uniffi.kshana_ffi.AshtakavargaRecord
import uniffi.kshana_ffi.ChartRecord
import uniffi.kshana_ffi.DashaRecord
import uniffi.kshana_ffi.DashaRow
import uniffi.kshana_ffi.ExactInstant
import uniffi.kshana_ffi.PanchangaDetail
import uniffi.kshana_ffi.dashaChain
import uniffi.kshana_ffi.dashaTable
import uniffi.kshana_ffi.varga
import com.kshana.app.ui.sky.KshanaFonts

private val VARGAS = listOf(
    1u, 2u, 3u, 4u, 5u, 6u, 7u, 8u, 9u, 10u, 11u, 12u, 16u, 20u, 24u, 27u, 30u, 40u, 45u, 60u,
    81u, 108u, 144u, 150u,
)
private val VARGA_NAMES = mapOf(
    1u to "Rasi", 2u to "Hora", 3u to "Drekkana", 4u to "Chaturthamsa", 5u to "Panchamsa",
    6u to "Shashthamsa", 7u to "Saptamsa", 8u to "Ashtamsa", 9u to "Navamsa", 10u to "Dasamsa",
    11u to "Rudramsa", 12u to "Dwadasamsa", 16u to "Shodasamsa", 20u to "Vimsamsa",
    24u to "Chaturvimsamsa", 27u to "Bhamsa", 30u to "Trimsamsa", 40u to "Khavedamsa",
    45u to "Akshavedamsa", 60u to "Shashtiamsa", 81u to "Navanavamsa", 108u to "Ashtottaramsa",
    144u to "Dwadas-dwadasamsa", 150u to "Nadiamsa",
)
private val VEDIC_ORDER = listOf("Sun", "Moon", "Mars", "Mercury", "Jupiter", "Venus", "Saturn")
private val LEVEL_NAMES = listOf("Maha", "Antar", "Pratyantar", "Sookshma", "Prana")

/** One body in the chosen divisional chart. */
private data class VargaPlacement(val name: String, val longitude: Double)

/** Everything the screen computes in one background pass. */
private data class VedicLoad(
    val chart: ChartRecord,
    val chain: List<DashaRecord>,
    val table: List<DashaRow>,
    val detail: PanchangaDetail,
    val ashtakavarga: AshtakavargaRecord,
)

/** Rasi (sidereal): North Indian chart, varga chooser, Vimshottari, birth panchanga. */
@Composable
fun VedicScreen(profile: BirthProfile, settings: EngineSettings) {
    val context = LocalContext.current
    var chart by remember { mutableStateOf<ChartRecord?>(null) }
    var chain by remember { mutableStateOf<List<DashaRecord>>(emptyList()) }
    var table by remember { mutableStateOf<List<DashaRow>>(emptyList()) }
    var detail by remember { mutableStateOf<PanchangaDetail?>(null) }
    var ashtakavarga by remember { mutableStateOf<AshtakavargaRecord?>(null) }
    var now by remember { mutableStateOf<ExactInstant?>(null) }
    var error by remember { mutableStateOf<String?>(null) }
    var vargaN by remember { mutableStateOf(1u) }
    var lagna by remember { mutableStateOf(0.0) }
    var placements by remember { mutableStateOf<List<VargaPlacement>>(emptyList()) }
    var tableOpen by remember { mutableStateOf(false) }
    var openMaha by remember { mutableIntStateOf(-1) }

    LaunchedEffect(profile, settings) {
        try {
            val engine = EngineProvider.get(context)
            val birth = profile.birthInstant()
            val at = Instants.now()
            val r = withContext(Dispatchers.Default) {
                val c = engine.chart(birth, profile.latitude, profile.longitude, 0.0, settings.houseSystem, settings.ayanamsa, settings.topocentricMoon)
                val moon = c.panchanga.moonSidereal
                VedicLoad(
                    chart = c,
                    chain = dashaChain(moon, birth, at, 3u),
                    table = dashaTable(moon, birth),
                    detail = engine.panchangaDetail(birth, profile.latitude, profile.longitude, settings.ayanamsa),
                    ashtakavarga = engine.ashtakavarga(birth, profile.latitude, profile.longitude, settings.ayanamsa),
                )
            }
            chart = r.chart
            chain = r.chain
            table = r.table
            detail = r.detail
            ashtakavarga = r.ashtakavarga
            now = at
            error = null
        } catch (e: Exception) {
            error = e.message ?: e.toString()
        }
    }

    // Divisional positions are recomputed off the main thread whenever the varga changes.
    LaunchedEffect(chart, vargaN) {
        val c = chart ?: return@LaunchedEffect
        try {
            val r = withContext(Dispatchers.Default) {
                val rahu = norm360(c.rahuTropical - c.ayanamsaDeg)
                val bodies = c.bodies.filter { it.name in VEDIC_ORDER }.map { VargaPlacement(it.name, varga(it.sidereal, vargaN)) } +
                    listOf(VargaPlacement("Rahu", varga(rahu, vargaN)), VargaPlacement("Ketu", varga(norm360(rahu + 180.0), vargaN)))
                Pair(varga(c.cuspsSidereal[0], vargaN), bodies)
            }
            lagna = r.first
            placements = r.second
        } catch (e: Exception) {
            error = e.message ?: e.toString()
        }
    }

    Column(
        Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(horizontal = 24.dp).padding(top = 48.dp, bottom = 32.dp),
        verticalArrangement = Arrangement.spacedBy(10.dp),
    ) {
        Text("RASI · SIDEREAL", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
        Text(profile.name, style = MaterialTheme.typography.displayMedium, color = KshanaColors.Text)
        Text(
            "${profile.birthLocal().format(DATE_TIME_S)} · ${profile.placeName} · ${settings.ayanamsa.name.lowercase().replace('_', '-').replaceFirstChar { it.uppercase() }}",
            style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted,
        )
        ErrorLine(error)
        val c = chart
        if (c == null) {
            Spacer(Modifier.height(320.dp))
            if (error == null) Working("casting the sky…")
        } else {
            Row(Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                for (n in VARGAS) Chip("D$n", selected = vargaN == n, onClick = { vargaN = n })
            }
            Text("${VARGA_NAMES[vargaN] ?: "D$vargaN"} · lagna ${degMin(lagna)} ${signName(lagna)}", style = MaterialTheme.typography.bodySmall, color = KshanaColors.Sky)
            Box(Modifier.fillMaxWidth(), contentAlignment = Alignment.Center) { NorthIndianChart(lagna, placements) }
            ashtakavarga?.let { AshtakavargaRow(it) }

            SectionLabel("Vimshottari")
            val at = now
            if (chain.isEmpty() || at == null) {
                Text("no running period", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted)
            } else {
                val maha = chain[0]
                for ((i, d) in chain.withIndex()) DashaLine(LEVEL_NAMES.getOrElse(i) { "L${d.level}" }, d)
                Text(
                    "${LEVEL_NAMES.getOrElse(chain.lastIndex) { "period" }} changes ${localText(chain.last().end, DATE_TIME_S)}",
                    fontFamily = KshanaFonts.mono, fontSize = 11.sp, color = KshanaColors.Muted,
                )
                Text(microText(chain.last().end), fontFamily = KshanaFonts.mono, fontSize = 11.sp, color = KshanaColors.Gold)
                ProgressBar(progress(maha.start, maha.end, at), chain.getOrNull(1)?.let { Pair(progress(maha.start, maha.end, it.start), progress(maha.start, maha.end, it.end)) })
                Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
                    Text(localText(maha.start, DATE_ONLY), fontFamily = KshanaFonts.mono, fontSize = 10.sp, color = KshanaColors.Muted)
                    Text("${maha.lord} mahadasha", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted)
                    Text(localText(maha.end, DATE_ONLY), fontFamily = KshanaFonts.mono, fontSize = 10.sp, color = KshanaColors.Muted)
                }
            }
            Row(
                Modifier.fillMaxWidth().height(44.dp).clickable { tableOpen = !tableOpen },
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.SpaceBetween,
            ) {
                Text("MAHADASHA TABLE", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
                Text(if (tableOpen) "hide" else "show", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted)
            }
            if (tableOpen) {
                val mahas = table.withIndex().filter { it.value.level.toInt() == 1 }
                for ((k, entry) in mahas.withIndex()) {
                    val m = entry.value
                    val running = at != null && at.seconds() >= m.start.seconds() && at.seconds() < m.end.seconds()
                    HorizontalDivider(color = KshanaColors.LineSoft)
                    Row(
                        Modifier.fillMaxWidth().heightIn(min = 44.dp).clickable { openMaha = if (openMaha == k) -1 else k }.padding(vertical = 6.dp),
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        Text(m.lord, fontFamily = KshanaFonts.serif, fontSize = 17.sp, color = if (running) KshanaColors.Gold else KshanaColors.Text, modifier = Modifier.weight(1f))
                        Text("${localText(m.start, DATE_ONLY)} → ${localText(m.end, DATE_ONLY)}", fontFamily = KshanaFonts.mono, fontSize = 11.sp, color = KshanaColors.Sky)
                    }
                    if (openMaha == k) {
                        val antars = table.drop(entry.index + 1).takeWhile { it.level.toInt() != 1 }
                        for (a in antars) {
                            val on = at != null && at.seconds() >= a.start.seconds() && at.seconds() < a.end.seconds()
                            Row(Modifier.fillMaxWidth().padding(start = 16.dp, top = 3.dp, bottom = 3.dp), verticalAlignment = Alignment.CenterVertically) {
                                Text("${m.lord}–${a.lord}", style = MaterialTheme.typography.bodyMedium, color = if (on) KshanaColors.Gold else KshanaColors.TextSoft, modifier = Modifier.weight(1f))
                                Text("${localText(a.start, DATE_ONLY)} → ${localText(a.end, DATE_ONLY)}", fontFamily = KshanaFonts.mono, fontSize = 10.sp, color = KshanaColors.Muted)
                            }
                        }
                    }
                }
                HorizontalDivider(color = KshanaColors.LineSoft)
            }

            SectionLabel("Panchanga at birth")
            detail?.let { PanchangaTiles(it, profile) }
            Text(
                "${c.provenanceEngine} · ${c.provenanceUt1} · ayanamsa ${"%.4f".format(c.ayanamsaDeg)}°",
                style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted, modifier = Modifier.padding(top = 12.dp),
            )
        }
    }
}

/**
 * Sarvashtakavarga: twelve cells from Aries, the lagna sign outlined in gold, a sign with
 * 28 bindus or more (the mean is 28) in gold figures.
 */
@Composable
private fun AshtakavargaRow(a: AshtakavargaRecord) {
    val lagna = a.lagnaSign.toInt()
    Column(Modifier.fillMaxWidth().padding(top = 4.dp)) {
        Text("SARVASHTAKAVARGA", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted)
        Row(Modifier.fillMaxWidth().padding(top = 6.dp), horizontalArrangement = Arrangement.spacedBy(2.dp)) {
            for (i in 0 until 12) {
                val bindus = a.sarva.getOrNull(i)?.toInt() ?: 0
                val isLagna = i == lagna
                val shape = RoundedCornerShape(4.dp)
                Column(
                    Modifier
                        .weight(1f)
                        .clip(shape)
                        .background(KshanaColors.Surface)
                        .border(1.dp, if (isLagna) KshanaColors.Gold else KshanaColors.LineSoft, shape)
                        .padding(vertical = 6.dp),
                    horizontalAlignment = Alignment.CenterHorizontally,
                ) {
                    Text(SIGN_SHORT[i].take(2), fontFamily = KshanaFonts.sans, fontSize = 8.sp, color = if (isLagna) KshanaColors.Gold else KshanaColors.Muted)
                    Text(bindus.toString(), fontFamily = KshanaFonts.mono, fontSize = 11.sp, color = if (bindus >= 28) KshanaColors.Gold else KshanaColors.Sky)
                }
            }
        }
        Text("bindus per sign from Aries · 28 is the mean · lagna outlined", style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted, modifier = Modifier.padding(top = 4.dp))
    }
}

@Composable
private fun DashaLine(level: String, d: DashaRecord) {
    Row(Modifier.fillMaxWidth().padding(vertical = 2.dp), verticalAlignment = Alignment.CenterVertically) {
        Text(level.uppercase(), style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted, modifier = Modifier.width(96.dp))
        Text(d.lord, fontFamily = KshanaFonts.serif, fontSize = 18.sp, color = KshanaColors.Text, modifier = Modifier.weight(1f))
        Text("${localText(d.start, DATE_ONLY)} → ${localText(d.end, DATE_ONLY)}", fontFamily = KshanaFonts.mono, fontSize = 11.sp, color = KshanaColors.Sky)
    }
}

/** Progress across the mahadasha, with the antardasha span marked in sky. */
@Composable
private fun ProgressBar(fraction: Float, antar: Pair<Float, Float>?) {
    Box(Modifier.fillMaxWidth().height(10.dp).clip(RoundedCornerShape(5.dp)).background(KshanaColors.LineSoft)) {
        if (antar != null) {
            Row(Modifier.fillMaxSize()) {
                Spacer(Modifier.weight(antar.first.coerceAtLeast(0.0001f)))
                Box(Modifier.weight((antar.second - antar.first).coerceAtLeast(0.0001f)).fillMaxHeight().background(KshanaColors.Sky.copy(alpha = 0.35f)))
                Spacer(Modifier.weight((1f - antar.second).coerceAtLeast(0.0001f)))
            }
        }
        Box(Modifier.fillMaxWidth(fraction.coerceAtLeast(0.002f)).fillMaxHeight().background(KshanaColors.Gold))
    }
}

@Composable
private fun PanchangaTiles(d: PanchangaDetail, profile: BirthProfile) {
    val zone = profile.birthZone()
    fun t(e: ExactInstant?): String = e?.let { localText(it, TIME_ONLY, zone) } ?: "—"
    Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(10.dp)) {
        VTile("Vara", d.vara, Modifier.weight(1f))
        VTile("Tithi", d.tithi, Modifier.weight(1f))
        VTile("Nakshatra", "${d.nakshatra} ${d.pada}", Modifier.weight(1f))
    }
    Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(10.dp)) {
        VTile("Yoga", d.yoga, Modifier.weight(1f))
        VTile("Karana", d.karana, Modifier.weight(1f))
        VTile("Hora", d.hora, Modifier.weight(1f))
    }
    Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(10.dp)) {
        VTile("Sunrise", t(d.sunrise), Modifier.weight(1f))
        VTile("Sunset", t(d.sunset), Modifier.weight(1f))
        VTile("Rahu kala", "${t(d.rahuKalaStart)} – ${t(d.rahuKalaEnd)}", Modifier.weight(1f))
    }
    Text(
        "tithi ends ${d.tithiEnds?.let { localText(it, DATE_TIME_S, zone) } ?: "—"} · nakshatra ends ${d.nakshatraEnds?.let { localText(it, DATE_TIME_S, zone) } ?: "—"}",
        fontFamily = KshanaFonts.mono, fontSize = 10.sp, color = KshanaColors.Muted,
    )
}

@Composable
private fun VTile(label: String, value: String, modifier: Modifier = Modifier) {
    Column(modifier.padding(top = 8.dp)) {
        Text(label.uppercase(), style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted)
        Text(value, fontFamily = KshanaFonts.serif, fontSize = 16.sp, color = KshanaColors.Text)
    }
}

/**
 * North Indian chart: a square with both diagonals and the inner diamond. House 1 is the
 * top diamond and houses run counter-clockwise. Signs follow the lagna; planets sit in
 * the house of their sign.
 */
@Composable
private fun NorthIndianChart(lagna: Double, placements: List<VargaPlacement>) {
    val measurer = rememberTextMeasurer()
    val numStyle = TextStyle(fontFamily = KshanaFonts.mono, fontSize = 9.sp, color = KshanaColors.Muted)
    val planetStyle = TextStyle(fontFamily = KshanaFonts.sans, fontSize = 12.sp, color = KshanaColors.Text)
    val lagnaStyle = TextStyle(fontFamily = KshanaFonts.sans, fontSize = 12.sp, color = KshanaColors.Gold)
    val lagnaSign = signIndex(lagna)
    val byHouse: Map<Int, List<String>> = placements.groupBy { ((signIndex(it.longitude) - lagnaSign + 12) % 12) + 1 }
        .mapValues { (_, v) -> v.map { PLANET_ABBR[it.name] ?: it.name.take(2) } }
    Canvas(Modifier.size(320.dp)) {
        val s = size.minDimension
        val o = Offset((size.width - s) / 2, (size.height - s) / 2)
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
        // sign-number anchors, nudged toward each cell's apex so planets have the centre
        val numbers = listOf(
            0.5 to 0.43, 0.25 to 0.03, 0.03 to 0.25, 0.43 to 0.5, 0.03 to 0.75, 0.25 to 0.97,
            0.5 to 0.57, 0.75 to 0.97, 0.97 to 0.75, 0.57 to 0.5, 0.97 to 0.25, 0.75 to 0.03,
        )
        for (h in 1..12) {
            val sign = (lagnaSign + h - 1) % 12 + 1
            val nl = measurer.measure(sign.toString(), numStyle)
            val np = p(numbers[h - 1].first, numbers[h - 1].second)
            drawText(nl, topLeft = Offset(np.x - nl.size.width / 2, np.y - nl.size.height / 2))
            val planetRows = (byHouse[h] ?: emptyList()).chunked(3)
            val rows: List<Pair<String, TextStyle>> =
                (if (h == 1) listOf("La" to lagnaStyle) else emptyList()) + planetRows.map { it.joinToString(" ") to planetStyle }
            if (rows.isEmpty()) continue
            val cp = p(centres[h - 1].first, centres[h - 1].second)
            val lineH = 14.sp.toPx()
            val top = cp.y - (rows.size - 1) * lineH / 2
            for ((ri, row) in rows.withIndex()) {
                val tl = measurer.measure(row.first, row.second)
                drawText(tl, topLeft = Offset(cp.x - tl.size.width / 2, top + ri * lineH - tl.size.height / 2))
            }
        }
    }
}
