using System;
using System.Collections.Generic;
using System.Text.Json.Serialization;

namespace Needlelight.Models;

// DTOs representing Thunderstore API responses
public class ThunderstorePackageDto
{
    [JsonPropertyName("name")] public string Name { get; set; } = string.Empty;
    [JsonPropertyName("full_name")] public string FullName { get; set; } = string.Empty;
    [JsonPropertyName("owner")] public string Owner { get; set; } = string.Empty;
    [JsonPropertyName("package_url")] public string PackageUrl { get; set; } = string.Empty;
    [JsonPropertyName("donation_link")] public string? DonationLink { get; set; }
    [JsonPropertyName("date_created")] public DateTime DateCreated { get; set; }
    [JsonPropertyName("date_updated")] public DateTime DateUpdated { get; set; }
    [JsonPropertyName("uuid4")] public Guid Uuid { get; set; }
    [JsonPropertyName("rating_score")] public int RatingScore { get; set; }
    [JsonPropertyName("is_pinned")] public bool IsPinned { get; set; }
    [JsonPropertyName("is_deprecated")] public bool IsDeprecated { get; set; }
    [JsonPropertyName("categories")] public List<string> Categories { get; set; } = new();
    [JsonPropertyName("versions")] public List<ThunderstorePackageVersionDto> Versions { get; set; } = new();
}

public class ThunderstorePackageVersionDto
{
    [JsonPropertyName("name")] public string Name { get; set; } = string.Empty;
    [JsonPropertyName("full_name")] public string FullName { get; set; } = string.Empty;
    [JsonPropertyName("description")] public string Description { get; set; } = string.Empty;
    [JsonPropertyName("icon")] public string Icon { get; set; } = string.Empty;
    [JsonPropertyName("version_number")] public string VersionNumber { get; set; } = string.Empty;
    [JsonPropertyName("dependencies")] public List<string> Dependencies { get; set; } = new();
    [JsonPropertyName("download_url")] public string DownloadUrl { get; set; } = string.Empty;
    [JsonPropertyName("downloads")] public int Downloads { get; set; }
    [JsonPropertyName("date_created")] public DateTime DateCreated { get; set; }
    [JsonPropertyName("website_url")] public string? WebsiteUrl { get; set; }
    [JsonPropertyName("is_active")] public bool IsActive { get; set; }
    [JsonPropertyName("file_size")] public long FileSize { get; set; }
}
