package com.kshana.app.net

import android.content.Context
import org.json.JSONArray
import org.json.JSONObject
import uniffi.kshana_ffi.SectionRecord
import java.io.IOException

/**
 * Optional narration: the engine's computed reading, sent to a Claude model with the user's own
 * key, comes back as a warm plain-language narrative. The model is told it may not add any
 * astrological claim, position, date or prediction that is not in the material it receives.
 * The user's name and exact birth time are not sent beyond what the report itself contains.
 */
object Narration {
    const val ENDPOINT = "https://api.anthropic.com/v1/messages"
    const val MODEL_SONNET = "claude-sonnet-5-5"
    const val MODEL_OPUS = "claude-opus-5-5"
    const val MODEL_HAIKU = "claude-haiku-5-5"
    const val DEFAULT_MODEL = MODEL_SONNET

    /** Model choices for the chips: label to model id. */
    val MODELS: List<Pair<String, String>> = listOf(
        "Sonnet" to MODEL_SONNET,
        "Opus" to MODEL_OPUS,
        "Haiku" to MODEL_HAIKU,
    )

    private const val MAX_TOKENS = 1500
    private const val REPORT_CAP = 60_000

    const val SYSTEM_PROMPT: String =
        "You are a narrator for an astrology app called Kshana. You receive the app's engine-computed " +
            "reading: a set of sections, each with plain sentences, a confidence for each sentence, a list of " +
            "reasons (why), and rule ids, plus a JSON report of the chart and every rule that fired. " +
            "Write a warm, plain-language narrative of about 300 to 500 words for the person whose reading " +
            "this is. Address the person as \"you\". " +
            "You must NOT add any astrological claim, planet position, date, timing or prediction that is not " +
            "in the provided material; do not invent transits, dashas, aspects or remedies. " +
            "When something is uncertain or has a low confidence, say so plainly rather than smoothing it over. " +
            "Do not use jargon without giving its plain meaning in the same breath. " +
            "If the person asks a question, answer it only from the material; if the material does not cover " +
            "it, say that the reading does not speak to it. " +
            "Write prose in short paragraphs, no headings, no bullet lists, no preamble."

    /**
     * Ask the model for a narrative. Throws [OnlineDisabled] when online is off, IOException with
     * the API's own error message on a non-2xx response, and IllegalStateException when the
     * response has no text.
     */
    suspend fun narrate(
        context: Context,
        apiKey: String,
        model: String,
        reportJson: String,
        sections: List<SectionRecord>,
        question: String?,
    ): String {
        Online.require(context)
        val key = apiKey.trim()
        if (key.isEmpty()) throw IllegalArgumentException("No narration key. Add one in Settings.")

        val userText = buildUserMessage(reportJson, sections, question)
        val body = JSONObject()
            .put("model", model.ifBlank { DEFAULT_MODEL })
            .put("max_tokens", MAX_TOKENS)
            .put("system", SYSTEM_PROMPT)
            .put(
                "messages",
                JSONArray().put(JSONObject().put("role", "user").put("content", userText)),
            )
            .toString()

        val headers = mapOf(
            "x-api-key" to key,
            "anthropic-version" to "2023-06-01",
            "content-type" to "application/json",
            "accept" to "application/json",
        )
        val (code, text) = Online.postJsonWithStatus(ENDPOINT, body, headers)
        if (code !in 200..299) throw IOException(apiErrorMessage(code, text))
        return parseText(text)
    }

    /** The sections rendered as text, then the (capped) report JSON, then the optional question. */
    internal fun buildUserMessage(reportJson: String, sections: List<SectionRecord>, question: String?): String {
        val sb = StringBuilder()
        sb.append("ENGINE READING (sections, sentences, confidence 0–1, why, rule ids)\n\n")
        for (s in sections) {
            sb.append("## ").append(s.title.trim()).append('\n')
            if (s.summary.isNotBlank()) sb.append(s.summary.trim()).append('\n')
            for (sen in s.sentences) {
                sb.append("- ").append(sen.text.trim())
                sb.append("  [confidence ").append("%.2f".format(sen.confidence)).append(']')
                if (sen.rules.isNotEmpty()) sb.append("  [rules: ").append(sen.rules.joinToString(", ")).append(']')
                sb.append('\n')
                for (w in sen.why) sb.append("    why: ").append(w.trim()).append('\n')
            }
            sb.append('\n')
        }
        sb.append("REPORT JSON (chart and fired rules; may be truncated at the end)\n")
        val trimmed = reportJson.trim()
        if (trimmed.length > REPORT_CAP) {
            sb.append(trimmed, 0, REPORT_CAP).append("\n…[truncated]\n")
        } else {
            sb.append(trimmed).append('\n')
        }
        val q = question?.trim().orEmpty()
        if (q.isNotEmpty()) {
            sb.append("\nQUESTION FROM THE PERSON (answer only from the material above)\n").append(q).append('\n')
        }
        return sb.toString()
    }

    /** Concatenate the text blocks of a Messages API response. */
    internal fun parseText(responseJson: String): String {
        val o = JSONObject(responseJson)
        val stop = o.optString("stop_reason")
        val content = o.optJSONArray("content") ?: JSONArray()
        val sb = StringBuilder()
        for (i in 0 until content.length()) {
            val block = content.optJSONObject(i) ?: continue
            if (block.optString("type") == "text") sb.append(block.optString("text"))
        }
        val text = sb.toString().trim()
        if (stop == "refusal" && text.isEmpty()) {
            throw IllegalStateException("The model declined to write this narration.")
        }
        if (text.isEmpty()) throw IllegalStateException("The model returned no text.")
        return text
    }

    /** The API's own error message when the body is JSON; otherwise the status code. */
    internal fun apiErrorMessage(code: Int, body: String): String {
        val fromJson = runCatching {
            val o = JSONObject(body)
            val err = o.optJSONObject("error")
            val msg = err?.optString("message").orEmpty()
            val type = err?.optString("type").orEmpty()
            when {
                msg.isNotEmpty() && type.isNotEmpty() -> "$type: $msg"
                msg.isNotEmpty() -> msg
                else -> ""
            }
        }.getOrDefault("")
        return if (fromJson.isNotEmpty()) "HTTP $code — $fromJson" else "HTTP $code from the narration service"
    }
}

suspend fun narrate(
    context: Context,
    apiKey: String,
    model: String,
    reportJson: String,
    sections: List<SectionRecord>,
    question: String?,
): String = Narration.narrate(context, apiKey, model, reportJson, sections, question)
