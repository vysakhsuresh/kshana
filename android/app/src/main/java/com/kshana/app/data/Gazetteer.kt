package com.kshana.app.data

import android.content.Context
import java.io.File
import java.io.RandomAccessFile
import java.util.zip.GZIPInputStream

/**
 * Every town and village in India and every town of 1,000 people or more elsewhere, from
 * GeoNames (CC BY 4.0), searchable offline. A birth in Aluva is read at Aluva's own
 * coordinates, not at the centre of Kochi twenty kilometres away.
 *
 * The list ships as a compressed, sorted table (assets/places.gz). The first search
 * unpacks it to the app's own storage once; after that a search is a binary search on
 * the sorted file, so the 700,000 names never sit in memory.
 *
 * Row: key, name, lat, lon, population, region index, zone index. The key is the name
 * folded to lowercase ASCII letters, digits and spaces; it is left empty when that fold
 * of the name itself gives it.
 */
object Gazetteer {
    private const val VERSION = 1
    private var offsets: IntArray? = null
    private var regions: List<String> = emptyList()
    private var zones: List<String> = emptyList()
    private var file: File? = null

    /** A found place, with its district and state (or region and country) for telling namesakes apart. */
    data class Hit(val name: String, val region: String, val latitude: Double, val longitude: Double, val zoneId: String, val population: Long) {
        fun place(): Place = Place(if (region.isBlank()) name else "$name, ${region.substringBefore(',')}", latitude, longitude, zoneId)
    }

    /** Folds a name the way the table was built: lowercase ASCII, digits and spaces only. */
    fun fold(s: String): String {
        val b = StringBuilder(s.length)
        for (c in s) {
            val l = if (c in 'A'..'Z') c + 32 else c
            if (l in 'a'..'z' || l in '0'..'9' || l == ' ') b.append(l)
        }
        return b.toString()
    }

    @Synchronized
    private fun open(context: Context) {
        if (offsets != null) return
        val dir = File(context.filesDir, "places").apply { mkdirs() }
        val f = File(dir, "places-v$VERSION.tsv")
        if (!f.exists()) {
            dir.listFiles()?.forEach { it.delete() }
            val tmp = File(dir, "unpacking.tmp")
            // The Android build unpacks a .gz asset and stores it as "places"; accept either.
            val plain = runCatching { context.assets.open("places") }.getOrNull()
            val input = plain ?: GZIPInputStream(context.assets.open("places.gz").buffered(1 shl 16))
            input.use { src -> tmp.outputStream().buffered(1 shl 16).use { src.copyTo(it, 1 shl 16) } }
            tmp.renameTo(f)
        }
        // Where each line starts.
        val starts = ArrayList<Int>(720_000)
        f.inputStream().buffered(1 shl 16).use { s ->
            var pos = 0
            starts.add(0)
            val buf = ByteArray(1 shl 16)
            while (true) {
                val n = s.read(buf)
                if (n <= 0) break
                for (i in 0 until n) if (buf[i] == '\n'.code.toByte()) starts.add(pos + i + 1)
                pos += n
            }
            if (starts.last() >= pos) starts.removeAt(starts.size - 1)
        }
        regions = context.assets.open("regions.tsv").bufferedReader().readLines()
        zones = context.assets.open("zones.tsv").bufferedReader().readLines()
        file = f
        offsets = starts.toIntArray()
    }

    private fun line(raf: RandomAccessFile, offs: IntArray, i: Int): String {
        val start = offs[i]
        val end = if (i + 1 < offs.size) offs[i + 1] - 1 else raf.length().toInt()
        val b = ByteArray((end - start).coerceAtLeast(0))
        raf.seek(start.toLong())
        raf.readFully(b)
        return String(b, Charsets.UTF_8)
    }

    private fun keyOf(cols: List<String>) = cols[0].ifEmpty { fold(cols[1]) }

    /** Up to [limit] places whose name starts with [query], exact names first, then the most populous. Call off the main thread. */
    fun search(context: Context, query: String, limit: Int = 25): List<Hit> {
        val q = fold(query).trim()
        if (q.length < 2) return emptyList()
        open(context)
        val offs = offsets ?: return emptyList()
        val f = file ?: return emptyList()
        RandomAccessFile(f, "r").use { raf ->
            // First line whose key is >= q.
            var lo = 0
            var hi = offs.size
            while (lo < hi) {
                val mid = (lo + hi) ushr 1
                if (keyOf(line(raf, offs, mid).split('\t')) < q) lo = mid + 1 else hi = mid
            }
            val found = ArrayList<Pair<String, Hit>>()
            var i = lo
            while (i < offs.size && found.size < 400) {
                val c = line(raf, offs, i).split('\t')
                val k = keyOf(c)
                if (!k.startsWith(q)) break
                if (c.size >= 7) {
                    val region = regions.getOrElse(c[5].toIntOrNull() ?: -1) { "" }
                    val lat = c[2].toDouble()
                    val lon = c[3].toDouble()
                    found.add(
                        k to Hit(
                            name = c[1],
                            region = region,
                            latitude = lat,
                            longitude = lon,
                            // Greater Bombay and Calcutta get their old city clock (same as IST after the switch).
                            zoneId = CityClocks.defaultZone(zones.getOrElse(c[6].ifEmpty { "0" }.toInt()) { "UTC" }, lat, lon, region),
                            population = c[4].toLongOrNull() ?: 0L,
                        ),
                    )
                }
                i++
            }
            return found
                .sortedWith(compareByDescending<Pair<String, Hit>> { it.first == q }.thenByDescending { it.second.population })
                .map { it.second }
                .distinctBy { "${it.name}|${it.region}|${"%.2f".format(it.latitude)}|${"%.2f".format(it.longitude)}" }
                .take(limit)
        }
    }
}
