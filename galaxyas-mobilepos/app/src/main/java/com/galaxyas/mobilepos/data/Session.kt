package com.galaxyas.mobilepos.data

import com.galaxyas.mobilepos.data.model.User
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow

/** Sesi login in-memory — mati aplikasi = login ulang (PIN cepat, tidak masalah). */
class Session {
    private val _user = MutableStateFlow<User?>(null)
    val user: StateFlow<User?> = _user.asStateFlow()

    fun setUser(u: User?) {
        _user.value = u
    }

    fun logout() {
        _user.value = null
    }

    /** Cek akses modul: master | penjualan | persediaan | laporan | pengaturan. Admin selalu boleh. */
    fun can(perm: String): Boolean {
        val u = _user.value ?: return false
        if (u.role == "admin") return true
        return perm in u.permissions
    }
}
