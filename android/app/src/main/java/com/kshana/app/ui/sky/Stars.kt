package com.kshana.app.ui.sky

import android.content.Context
import java.io.ByteArrayOutputStream
import java.nio.ByteBuffer
import java.nio.ByteOrder
import kotlin.math.PI
import kotlin.math.asin
import kotlin.math.atan2
import kotlin.math.cos
import kotlin.math.sin

/*
 * Real stars for the living sky.
 *
 * Data: the Yale Bright Star Catalogue (BSC5) to visual magnitude 5.0, packed offline into
 * assets/sky/bsc.bin (see the README there): 1 630 stars with J2000 RA/Dec, V magnitude,
 * colour temperature and the HR number, plus the common names of the 64 brightest named
 * stars. Everything here is plain arithmetic on float arrays so a projection of the whole
 * catalogue allocates nothing after construction.
 *
 * Geometry: J2000 positions are precessed to the date with the IAU 2006 equatorial
 * precession angles (ζ, z, θ; Capitaine et al. 2003), then turned into the observer's
 * horizon frame from the engine's local sidereal time (hour angle H = LST − RA) and the
 * place latitude, and projected stereographically from the point on the horizon the
 * observer faces (south north of the equator), so the horizon is a straight line and the
 * east and west points sit at the ends of the ecliptic ring.
 */

/** The packed catalogue, brightest first. Angles in degrees. */
class StarCatalogue(
    val count: Int,
    val raDeg: FloatArray,
    val decDeg: FloatArray,
    val mag: FloatArray,
    /** Colour temperature in hundreds of kelvin, 0 unknown. */
    val tempHundreds: ShortArray,
    /** 0 warm, 1 white, 2 blue, from the temperature. */
    val tint: ByteArray,
    val hr: IntArray,
    /** Common name, or null. */
    val name: Array<String?>,
) {
    /** Indices of the named stars, brightest first. */
    val namedIndices: IntArray = (0 until count).filter { name[it] != null }.toIntArray()

    /** Unit vectors in the J2000 equatorial frame. */
    val ux = FloatArray(count)
    val uy = FloatArray(count)
    val uz = FloatArray(count)

    init {
        for (i in 0 until count) {
            val ra = Math.toRadians(raDeg[i].toDouble())
            val dec = Math.toRadians(decDeg[i].toDouble())
            val cd = cos(dec)
            ux[i] = (cd * cos(ra)).toFloat()
            uy[i] = (cd * sin(ra)).toFloat()
            uz[i] = sin(dec).toFloat()
        }
    }
}

object Stars {
    const val ASSET = "sky/bsc.bin"

    /** Sidereal rate: degrees of LST per SI second. */
    const val LST_DEG_PER_SECOND = 360.98564736629 / 86400.0

    @Volatile private var cached: StarCatalogue? = null

    /** The catalogue from the APK's assets, parsed once per process; null if the asset is missing. */
    fun load(context: Context): StarCatalogue? {
        cached?.let { return it }
        synchronized(this) {
            cached?.let { return it }
            val bytes = runCatching {
                context.applicationContext.assets.open(ASSET).use { input ->
                    val out = ByteArrayOutputStream(16 * 1024)
                    input.copyTo(out)
                    out.toByteArray()
                }
            }.getOrNull() ?: return null
            val parsed = runCatching { parse(bytes) }.getOrNull() ?: return null
            cached = parsed
            return parsed
        }
    }

    /**
     * Parse bsc.bin: "KSKY", u8 version, u8 reserved, u16 count; count × (u16 ra, i16 dec,
     * u8 mag, u8 temp, u16 hr); u16 named; named × (u16 index, u8 len, bytes).
     */
    fun parse(bytes: ByteArray): StarCatalogue {
        val buf = ByteBuffer.wrap(bytes).order(ByteOrder.LITTLE_ENDIAN)
        require(buf.get() == 'K'.code.toByte() && buf.get() == 'S'.code.toByte() && buf.get() == 'K'.code.toByte() && buf.get() == 'Y'.code.toByte()) { "not a sky pack" }
        val version = buf.get().toInt()
        require(version == 1) { "sky pack version $version" }
        buf.get()
        val count = buf.short.toInt() and 0xFFFF
        val ra = FloatArray(count)
        val dec = FloatArray(count)
        val mag = FloatArray(count)
        val temp = ShortArray(count)
        val tint = ByteArray(count)
        val hr = IntArray(count)
        for (i in 0 until count) {
            ra[i] = ((buf.short.toInt() and 0xFFFF) * (360.0 / 65536.0)).toFloat()
            dec[i] = (buf.short.toInt() * (90.0 / 32767.0)).toFloat()
            mag[i] = ((buf.get().toInt() and 0xFF) / 20f) - 2f
            val t = buf.get().toInt() and 0xFF
            temp[i] = t.toShort()
            tint[i] = tintOf(t).toByte()
            hr[i] = buf.short.toInt() and 0xFFFF
        }
        val names = arrayOfNulls<String>(count)
        if (buf.remaining() >= 2) {
            val named = buf.short.toInt() and 0xFFFF
            for (k in 0 until named) {
                val idx = buf.short.toInt() and 0xFFFF
                val len = buf.get().toInt() and 0xFF
                val b = ByteArray(len)
                buf.get(b)
                if (idx < count) names[idx] = String(b, Charsets.US_ASCII)
            }
        }
        return StarCatalogue(count, ra, dec, mag, temp, tint, hr, names)
    }

    /** Warm below 5 000 K (Betelgeuse, Aldebaran, Arcturus), blue above 7 500 K (Vega, Sirius, Rigel). */
    fun tintOf(tempHundreds: Int): Int = when {
        tempHundreds == 0 -> 1
        tempHundreds < 50 -> 0
        tempHundreds > 75 -> 2
        else -> 1
    }

    /**
     * The IAU 2006 precession matrix from J2000 to the date [t] (Julian centuries TT since
     * J2000), row-major into [out] (9 doubles): v_date = P · v_J2000.
     */
    fun precessionMatrix(t: Double, out: DoubleArray) {
        val arc = PI / 648000.0 // arcseconds to radians
        val t2 = t * t
        val t3 = t2 * t
        val t4 = t3 * t
        val t5 = t4 * t
        val zeta = (2.650545 + 2306.083227 * t + 0.2988499 * t2 + 0.01801828 * t3 - 0.000005971 * t4 - 0.0000003173 * t5) * arc
        val z = (-2.650545 + 2306.077181 * t + 1.0927348 * t2 + 0.01826837 * t3 - 0.000028596 * t4 - 0.0000002904 * t5) * arc
        val theta = (2004.191903 * t - 0.4294934 * t2 - 0.04182264 * t3 - 0.000007089 * t4 - 0.0000001274 * t5) * arc
        val cz = cos(zeta); val sz = sin(zeta)
        val cZ = cos(z); val sZ = sin(z)
        val ct = cos(theta); val st = sin(theta)
        out[0] = cz * ct * cZ - sz * sZ
        out[1] = -sz * ct * cZ - cz * sZ
        out[2] = -st * cZ
        out[3] = cz * ct * sZ + sz * cZ
        out[4] = -sz * ct * sZ + cz * cZ
        out[5] = -st * sZ
        out[6] = cz * st
        out[7] = -sz * st
        out[8] = ct
    }

    /** The identity, for when no date is known. */
    fun identity(out: DoubleArray) {
        for (i in 0 until 9) out[i] = if (i % 4 == 0) 1.0 else 0.0
    }

    /**
     * Ecliptic longitude/latitude of date to right ascension/declination of date, degrees,
     * written into [out] as (ra, dec).
     */
    fun eclipticToEquatorial(lonDeg: Double, latDeg: Double, obliquityDeg: Double, out: DoubleArray) {
        val l = Math.toRadians(lonDeg)
        val b = Math.toRadians(latDeg)
        val e = Math.toRadians(obliquityDeg)
        val sinDec = sin(b) * cos(e) + cos(b) * sin(e) * sin(l)
        val y = sin(l) * cos(e) * cos(b) - sin(b) * sin(e)
        val x = cos(l) * cos(b)
        out[0] = (Math.toDegrees(atan2(y, x)) + 360.0) % 360.0
        out[1] = Math.toDegrees(asin(sinDec.coerceIn(-1.0, 1.0)))
    }

    /** Geometric altitude in degrees of a body at (ra, dec) of date for an observer at [latDeg] with local sidereal time [lstDeg]. */
    fun altitudeDeg(raDeg: Double, decDeg: Double, lstDeg: Double, latDeg: Double): Double {
        val h = Math.toRadians(lstDeg - raDeg)
        val d = Math.toRadians(decDeg)
        val phi = Math.toRadians(latDeg)
        val sinAlt = sin(phi) * sin(d) + cos(phi) * cos(d) * cos(h)
        return Math.toDegrees(asin(sinAlt.coerceIn(-1.0, 1.0)))
    }
}

/**
 * The catalogue projected onto a canvas, kept in fixed arrays that [project] rewrites in
 * place: the bright stars (V < [BRIGHT_LIMIT]) as individual discs with their own twinkle,
 * the faint ones batched into twelve point buckets (four magnitude bands × three tints) for
 * `drawRawPoints`. Invisible stars are parked off-canvas so the arrays never change size.
 */
class StarProjection(val cat: StarCatalogue) {
    companion object {
        const val BRIGHT_LIMIT = 3.0f
        const val BANDS = 4
        const val TINTS = 3
        const val BUCKETS = BANDS * TINTS
        private const val OFF = -1000f

        /** Magnitude band of a faint star: 3.0–3.5, 3.5–4.0, 4.0–4.5, 4.5–5.0+. */
        fun band(mag: Float): Int = ((mag - BRIGHT_LIMIT) * 2f).toInt().coerceIn(0, BANDS - 1)

        /** Mean magnitude of a band, for its alpha and point size. */
        fun bandMag(band: Int): Float = BRIGHT_LIMIT + 0.25f + 0.5f * band

        /** Radius in dp for a magnitude: Sirius ≈ 2.6, mag 1 ≈ 1.9, mag 3 ≈ 1.3, mag 5 ≈ 0.7. */
        fun radiusDp(mag: Float): Float = (0.7f + (5f - mag) * 0.3f).coerceIn(0.6f, 3f)

        /** Base alpha for a magnitude: 1 for the brightest, ~0.37 at mag 5. */
        fun baseAlpha(mag: Float): Float = 0.35f + 0.65f * ((5.2f - mag) / 6.5f).coerceIn(0f, 1f)

        /** How visible a star of [mag] is under a limiting magnitude [limit]: fades in over 0.8 mag. */
        fun fade(mag: Float, limit: Float): Float = ((limit - mag + 0.3f) / 0.8f).coerceIn(0f, 1f)
    }

    /** Number of bright (disc) stars; they are the first [nBright] catalogue entries. */
    val nBright: Int = run {
        var n = 0
        while (n < cat.count && cat.mag[n] < BRIGHT_LIMIT) n++
        n
    }

    val bx = FloatArray(nBright)
    val by = FloatArray(nBright)
    val bAlpha = FloatArray(nBright)
    val bTwinkle = FloatArray(nBright)
    val bPhase = FloatArray(nBright) { ((cat.hr[it] * 0.6180339f) % 1f) * 6.2831855f }
    val bVisible = BooleanArray(nBright)

    /** Point buckets: x,y pairs. */
    val pts: Array<FloatArray>
    /** Per-bucket alpha after the limiting magnitude, moonlight and intensity. */
    val bucketAlpha = FloatArray(BUCKETS)
    private val slotBucket = IntArray(cat.count)
    private val slotIndex = IntArray(cat.count)

    /** Stars above the horizon and inside the canvas at the last projection. */
    var visibleCount = 0
        private set

    /** The size and sidereal time of the last projection, so callers can skip redundant work. */
    var lastWidth = -1f
        private set
    var lastHeight = -1f
        private set
    var lastLstDeg = Double.NaN
        private set

    private val m = DoubleArray(9)

    init {
        val counts = IntArray(BUCKETS)
        for (i in nBright until cat.count) {
            val b = band(cat.mag[i]) * TINTS + cat.tint[i].toInt().coerceIn(0, TINTS - 1)
            slotBucket[i] = b
            slotIndex[i] = counts[b]++
        }
        pts = Array(BUCKETS) { b -> FloatArray(2 * counts[b]) { OFF } }
    }

    /**
     * Rewrite every position. [precession] is the 3×3 J2000→date matrix; [lstDeg] the local
     * sidereal time; [latDeg] the place latitude; [southern] flips east to the right; the
     * horizon is the line y = [horizonY] with the facing point at x = [cx] and the east/west
     * points [scale] pixels either side; [limitingMag] is the faintest magnitude the sky lets
     * through now, [moonDim] a factor for moonlight (1 = none), [intensity] the caller's.
     */
    fun project(
        precession: DoubleArray,
        lstDeg: Double,
        latDeg: Double,
        southern: Boolean,
        cx: Float,
        horizonY: Float,
        scale: Float,
        width: Float,
        height: Float,
        limitingMag: Float,
        moonDim: Float,
        intensity: Float,
    ) {
        lastWidth = width
        lastHeight = height
        lastLstDeg = lstDeg
        // Horizon frame rows (east, south, up) from the equatorial frame of date, times precession.
        val l = Math.toRadians(lstDeg)
        val phi = Math.toRadians(latDeg)
        val cl = cos(l); val sl = sin(l)
        val cp = cos(phi); val sp = sin(phi)
        val e0 = -sl; val e1 = cl; val e2 = 0.0
        val s0 = sp * cl; val s1 = sp * sl; val s2 = -cp
        val u0 = cp * cl; val u1 = cp * sl; val u2 = sp
        val p = precession
        // m = H · P
        m[0] = e0 * p[0] + e1 * p[3] + e2 * p[6]; m[1] = e0 * p[1] + e1 * p[4] + e2 * p[7]; m[2] = e0 * p[2] + e1 * p[5] + e2 * p[8]
        m[3] = s0 * p[0] + s1 * p[3] + s2 * p[6]; m[4] = s0 * p[1] + s1 * p[4] + s2 * p[7]; m[5] = s0 * p[2] + s1 * p[5] + s2 * p[8]
        m[6] = u0 * p[0] + u1 * p[3] + u2 * p[6]; m[7] = u0 * p[1] + u1 * p[4] + u2 * p[7]; m[8] = u0 * p[2] + u1 * p[5] + u2 * p[8]
        val m0 = m[0].toFloat(); val m1 = m[1].toFloat(); val m2 = m[2].toFloat()
        val m3 = m[3].toFloat(); val m4 = m[4].toFloat(); val m5 = m[5].toFloat()
        val m6 = m[6].toFloat(); val m7 = m[7].toFloat(); val m8 = m[8].toFloat()
        val dir = if (southern) -1f else 1f
        val margin = 6f
        var visible = 0

        val ux = cat.ux; val uy = cat.uy; val uz = cat.uz; val mag = cat.mag
        for (i in 0 until cat.count) {
            val x = ux[i]; val y = uy[i]; val z = uz[i]
            val e = m0 * x + m1 * y + m2 * z
            val s = m3 * x + m4 * y + m5 * z
            val u = m6 * x + m7 * y + m8 * z
            val f = dir * s
            var px = OFF
            var py = OFF
            var alpha = 0f
            if (u >= 0f && f > -0.75f) {
                val d = 1f + f
                px = cx + scale * (-dir * e) / d
                py = horizonY - scale * u / d
                if (px < -margin || px > width + margin || py < -margin || py > horizonY + margin) {
                    px = OFF; py = OFF
                } else {
                    // Extinction near the horizon, then the sky's limiting magnitude and moonlight.
                    val horizonDim = 0.45f + 0.55f * (u * 4f).coerceIn(0f, 1f)
                    alpha = baseAlpha(mag[i]) * fade(mag[i], limitingMag) * horizonDim * moonDim * intensity
                    if (alpha > 0.004f) visible++ else { px = OFF; py = OFF; alpha = 0f }
                }
            }
            if (i < nBright) {
                bx[i] = px; by[i] = py; bAlpha[i] = alpha
                bVisible[i] = px != OFF
                // Stars near the horizon scintillate more: amplitude 0.08 at the zenith, 0.28 at the horizon.
                bTwinkle[i] = 0.08f + 0.2f * (1f - u.coerceIn(0f, 1f))
            } else {
                val arr = pts[slotBucket[i]]
                val k = 2 * slotIndex[i]
                arr[k] = px
                arr[k + 1] = py
            }
        }
        for (b in 0 until BUCKETS) {
            val bm = bandMag(b / TINTS)
            bucketAlpha[b] = baseAlpha(bm) * fade(bm, limitingMag) * moonDim * intensity * 0.85f
        }
        visibleCount = visible
    }

    /** Park everything off-canvas (no catalogue drawn). */
    fun clear() {
        for (i in 0 until nBright) { bx[i] = OFF; by[i] = OFF; bAlpha[i] = 0f; bVisible[i] = false }
        for (b in 0 until BUCKETS) { val arr = pts[b]; for (k in arr.indices) arr[k] = OFF; bucketAlpha[b] = 0f }
        visibleCount = 0
        lastWidth = -1f
        lastHeight = -1f
        lastLstDeg = Double.NaN
    }
}
