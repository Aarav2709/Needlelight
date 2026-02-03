using System;

namespace Needlelight.Models;

/// <summary>
/// Represents the response from https://thunderstore.io/api/experimental/package/{namespace}/{name}/
/// Only includes fields needed for update checks.
/// </summary>
public class ThunderstorePackageDetailDto
{
    public string Namespace { get; set; } = string.Empty;
    public string Name { get; set; } = string.Empty;
    public string Full_Name { get; set; } = string.Empty; // author-ModName
    public DateTime Date_Updated { get; set; }

    public LatestDto? Latest { get; set; }

    public class LatestDto
    {
        public string Version_Number { get; set; } = string.Empty;
        public string Download_Url { get; set; } = string.Empty;
        public DateTime Date_Created { get; set; }
        public int Downloads { get; set; }
        public string? Icon { get; set; }
        public string? Description { get; set; }
    }
}
