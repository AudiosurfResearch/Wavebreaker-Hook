/**
 * This file:
 * - hooks wininet functions
 * - rewrites game requests to go to a custom server
 * - rewrites HTTP requests to use HTTPS
 */

#include <Windows.h>
#include <WinInet.h>
#include <stdexcept>
#include <safetyhook.hpp>
#include <spdlog/spdlog.h>

namespace wavebreaker
{
    namespace nethook
    {
        SafetyHookInline g_getserverunicode_hook{};
        SafetyHookInline g_getserver_hook{};
        SafetyHookInline g_internetconnecta_hook{};
        SafetyHookInline g_httpopenrequesta_hook{};

        char *rewriteTargetServer(char *server)
        {
            if (strstr(server, "audio-surf") || strstr(server, "audiosurfthegame"))
                server = _strdup(config::server.c_str());
            return server;
        }

        char *__fastcall GetTargetServerUnicodeHook(void *thisptr, uintptr_t edx)
        {
            char *server = g_getserverunicode_hook.thiscall<char *>(thisptr);
            spdlog::debug("Rewriting server (Unicode): {0}", server);
            return rewriteTargetServer(server);
        }

        char *__fastcall GetTargetServerHook(void *thisptr, uintptr_t edx)
        {
            char *server = g_getserver_hook.thiscall<char *>(thisptr);
            spdlog::debug("Rewriting server: {0}", server);
            return rewriteTargetServer(server);
        }

        HINTERNET WINAPI InternetConnectHook(HINTERNET hInternet,
                                             LPCSTR lpszServerName,
                                             INTERNET_PORT nServerPort,
                                             LPCSTR lpszUserName,
                                             LPCSTR lpszPassword,
                                             DWORD dwService,
                                             DWORD dwFlags,
                                             DWORD_PTR dwContext)
        {
            spdlog::debug("InternetConnect hook hit: {0} {1}", lpszServerName, nServerPort);
            if (nServerPort == 80 && !config::forceInsecure)
                nServerPort = 443;
            if (nServerPort == 443 && config::forceInsecure)
                nServerPort = 80;
            return g_internetconnecta_hook.stdcall<HINTERNET>(hInternet, lpszServerName, nServerPort, lpszUserName, lpszPassword, dwService, dwFlags, dwContext);
        }

        HINTERNET WINAPI OpenRequestHook(HINTERNET hConnect,
                                         LPCSTR lpszVerb,
                                         LPCSTR lpszObjectName,
                                         LPCSTR lpszVersion,
                                         LPCSTR lpszReferrer,
                                         LPCSTR *lplpszAcceptTypes,
                                         DWORD dwFlags,
                                         DWORD_PTR dwContext)
        {
            if (config::forceInsecure)
            {
                dwFlags &= ~INTERNET_FLAG_SECURE;
            }
            else
            {
                if (!dwFlags)
                    dwFlags = INTERNET_FLAG_SECURE;
                if (dwFlags == INTERNET_FLAG_RELOAD)
                    dwFlags = INTERNET_FLAG_RELOAD | INTERNET_FLAG_SECURE;
            }

            spdlog::debug("OpenRequest hook hit: {0} {1} {2} {3}", lpszVersion, lpszVerb, lpszReferrer, lpszObjectName);
            return g_httpopenrequesta_hook.stdcall<HINTERNET>(hConnect,
                                                              lpszVerb,
                                                              lpszObjectName,
                                                              lpszVersion,
                                                              lpszReferrer,
                                                              lplpszAcceptTypes,
                                                              dwFlags,
                                                              dwContext);
        }

        void init()
        {
            spdlog::info("Attaching networking hooks");
            g_internetconnecta_hook = safetyhook::create_inline((void *)InternetConnectA, (void *)InternetConnectHook);
            g_httpopenrequesta_hook = safetyhook::create_inline((void *)HttpOpenRequestA, (void *)OpenRequestHook);

            FARPROC targetServerUnicodeHandle;
            FARPROC targetServerHandle;
            int tryCount = 0;

            while ((!g_getserverunicode_hook || !g_getserverunicode_hook) && tryCount < 5)
            {
                spdlog::debug("Network hook attempt {0}", tryCount);
                Sleep(75);
                targetServerUnicodeHandle = GetProcAddress(GetModuleHandleA("HTTP_Fetch_Unicode.dll"), "?GetTargetServer@HTTP_Fetch_Unicode@@UAEPADXZ");
                targetServerHandle = GetProcAddress(GetModuleHandleA("17C5B19F-4273-423C-A158-CA6F73046D43.dll"), "?GetTargetServer@Aco_HTTP_Fetch@@UAEPADXZ");
                spdlog::debug("Networking channel function handles: {0:p}, {1:p}", fmt::ptr(targetServerUnicodeHandle), fmt::ptr(targetServerHandle));

                g_getserverunicode_hook = safetyhook::create_inline((void *)targetServerUnicodeHandle, (void *)GetTargetServerUnicodeHook);
                g_getserver_hook = safetyhook::create_inline((void *)targetServerHandle, (void *)GetTargetServerHook);

                tryCount++;
            }

            if (!g_getserver_hook || !g_getserverunicode_hook || !g_internetconnecta_hook || !g_httpopenrequesta_hook)
            {
                spdlog::critical("Failed to attach hook(s). Hook destinations are: {0:p} {1:p} {2:p} {3:p}, hook targets are: {4:p}, {5:p}, {6:p}, {7:p}",
                                 fmt::ptr(g_getserver_hook.destination()),
                                 fmt::ptr(g_getserverunicode_hook.destination()),
                                 fmt::ptr(g_internetconnecta_hook.destination()),
                                 fmt::ptr(g_httpopenrequesta_hook.destination()),
                                 fmt::ptr(g_getserver_hook.target()),
                                 fmt::ptr(g_getserverunicode_hook.target()),
                                 fmt::ptr(g_internetconnecta_hook.target()),
                                 fmt::ptr(g_httpopenrequesta_hook.target()));
                throw std::runtime_error("Hook failed");
            }
        }
    }
}