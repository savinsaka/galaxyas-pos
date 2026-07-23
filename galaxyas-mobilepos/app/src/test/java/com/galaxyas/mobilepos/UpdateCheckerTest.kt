package com.galaxyas.mobilepos

import com.galaxyas.mobilepos.data.network.UpdateChecker
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class UpdateCheckerTest {
    @Test
    fun `versi lebih besar terdeteksi baru`() {
        assertTrue(UpdateChecker.isNewer("0.2.0", "0.1.0"))
        assertTrue(UpdateChecker.isNewer("1.0.0", "0.9.9"))
        assertTrue(UpdateChecker.isNewer("0.1.1", "0.1.0"))
        assertTrue(UpdateChecker.isNewer("v0.2.0", "0.1.0"))
    }

    @Test
    fun `versi sama atau lebih lama tidak dianggap baru`() {
        assertFalse(UpdateChecker.isNewer("0.1.0", "0.1.0"))
        assertFalse(UpdateChecker.isNewer("0.1.0", "0.2.0"))
        assertFalse(UpdateChecker.isNewer("0.9.9", "1.0.0"))
    }

    @Test
    fun `format tak lengkap ditoleransi`() {
        assertTrue(UpdateChecker.isNewer("1.1", "1.0.9"))
        assertFalse(UpdateChecker.isNewer("1", "1.0.0"))
    }
}
