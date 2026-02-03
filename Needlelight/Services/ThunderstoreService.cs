using System;
using System.Collections.Generic;
using System.Net.Http;
using System.Net.Http.Json;
using System.Threading;
using System.Threading.Tasks;
using Microsoft.Extensions.Caching.Memory;
using Needlelight.Models;

namespace Needlelight.Services;

public class ThunderstoreService : IDisposable
{
    private readonly HttpClient _http;
    private readonly IMemoryCache _cache;
    private readonly SemaphoreSlim _throttle = new(1, 1);
    private readonly TimeSpan _cacheTtl = TimeSpan.FromMinutes(15);
    private readonly string _baseUrl;
    private readonly bool _ownsHttpClient;
    private readonly bool _ownsCache;

    public ThunderstoreService(HttpClient? http = null, IMemoryCache? cache = null, string? baseUrl = null)
    {
        _ownsHttpClient = http == null;
        _ownsCache = cache == null;
        _http = http ?? new HttpClient();
        _cache = cache ?? new MemoryCache(new MemoryCacheOptions());
        _baseUrl = baseUrl ?? Environment.GetEnvironmentVariable("THUNDERSTORE_BASE_URL")?.TrimEnd('/') ?? "https://thunderstore.io";
    }

    public async Task<IReadOnlyList<ThunderstorePackageDto>> GetPackagesAsync(int page = 1, CancellationToken token = default)
    {
        var cacheKey = $"packages_page_{page}";
        if (_cache.TryGetValue(cacheKey, out IReadOnlyList<ThunderstorePackageDto>? cached))
        {
            return cached ?? new List<ThunderstorePackageDto>();
        }

        await _throttle.WaitAsync(token);
        try
        {
            if (_cache.TryGetValue(cacheKey, out cached))
            {
                return cached ?? new List<ThunderstorePackageDto>();
            }

            var url = $"{_baseUrl}/c/hollow-knight-silksong/api/v1/package/?page={page}";
            var response = await _http.GetAsync(url, token);
            response.EnsureSuccessStatusCode();
            var packages = await response.Content.ReadFromJsonAsync<List<ThunderstorePackageDto>>(cancellationToken: token) ?? new();
            _cache.Set(cacheKey, packages, _cacheTtl);
            return packages;
        }
        finally
        {
            _throttle.Release();
        }
    }

    public async Task<ThunderstorePackageDetailDto?> GetPackageDetailAsync(string ns, string name, bool force = false, CancellationToken token = default)
    {
        var cacheKey = $"pkg_{ns}_{name}";
        if (!force && _cache.TryGetValue(cacheKey, out ThunderstorePackageDetailDto? cached))
        {
            return cached;
        }

        await _throttle.WaitAsync(token);
        try
        {
            if (!force && _cache.TryGetValue(cacheKey, out cached))
                return cached;

            var url = $"{_baseUrl}/api/experimental/package/{ns}/{name}/";
            var response = await _http.GetAsync(url, token);
            response.EnsureSuccessStatusCode();
            var detail = await response.Content.ReadFromJsonAsync<ThunderstorePackageDetailDto>(cancellationToken: token);
            if (detail != null)
                _cache.Set(cacheKey, detail, _cacheTtl);
            return detail;
        }
        finally
        {
            _throttle.Release();
        }
    }

    public void Dispose()
    {
        if (_ownsHttpClient)
            _http.Dispose();
        if (_ownsCache)
            _cache.Dispose();
        _throttle.Dispose();
    }
}
