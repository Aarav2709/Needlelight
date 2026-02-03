using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Net.Http;
using System.Threading;
using System.Threading.Tasks;
using System.Xml;
using System.Xml.Serialization;
using Microsoft.Extensions.Caching.Memory;
using Needlelight.Interfaces;
using Needlelight.Models;

namespace Needlelight.Services;

public static class ThunderstoreModLinksBuilder
{
    public static async Task<ModLinks?> BuildAsync(HttpClient hc, ISettings settings, CancellationToken token = default)
    {
        try
        {
            _ = settings;
            using var cache = new MemoryCache(new MemoryCacheOptions());
            using var api = new ThunderstoreService(hc, cache);

            var packages = await api.GetPackagesAsync(1, token);
            if (packages.Count == 0)
                return new ModLinks { Manifests = Array.Empty<Manifest>(), Raw = string.Empty };

            var availableNames = new HashSet<string>(packages.Select(p => p.FullName), StringComparer.OrdinalIgnoreCase);

            var manifests = packages
                .Select(pkg => MapPackage(pkg, availableNames))
                .Where(m => m != null)
                .Select(m => m!)
                .ToArray();

            var modLinks = new ModLinks { Manifests = manifests };
            modLinks.Raw = BuildRawXml(manifests);
            return modLinks;
        }
        catch
        {
            return null;
        }
    }

    private static Manifest? MapPackage(ThunderstorePackageDto pkg, HashSet<string> availableNames)
    {
        if (pkg.Versions.Count == 0)
            return null;

        var latest = pkg.Versions.OrderByDescending(v => v.DateCreated).First();
        var version = ParseVersion(latest.VersionNumber);
        var dependencies = NormalizeDependencies(latest.Dependencies, availableNames);
        var link = new Link { SHA256 = string.Empty, URL = latest.DownloadUrl };

        var repository = !string.IsNullOrWhiteSpace(latest.WebsiteUrl)
            ? latest.WebsiteUrl
            : pkg.PackageUrl;

        return new Manifest
        {
            Version = version,
            Name = string.IsNullOrWhiteSpace(pkg.FullName) ? pkg.Name : pkg.FullName,
            Repository = repository ?? string.Empty,
            Issues = repository ?? string.Empty,
            Links = new Links
            {
                Windows = link,
                Mac = link,
                Linux = link
            },
            Dependencies = dependencies,
            Description = latest.Description ?? string.Empty,
            Tags = pkg.Categories?.ToArray() ?? Array.Empty<string>(),
            Integrations = Array.Empty<string>(),
            Authors = string.IsNullOrWhiteSpace(pkg.Owner) ? Array.Empty<string>() : new[] { pkg.Owner }
        };
    }

    private static string[] NormalizeDependencies(IEnumerable<string> dependencies, HashSet<string> availableNames)
    {
        var result = new List<string>();
        foreach (var dep in dependencies)
        {
            var normalized = NormalizeDependencyName(dep);
            if (string.IsNullOrWhiteSpace(normalized))
                continue;

            if (availableNames.Contains(normalized))
                result.Add(normalized);
        }
        return result.ToArray();
    }

    private static string NormalizeDependencyName(string dependency)
    {
        if (string.IsNullOrWhiteSpace(dependency))
            return string.Empty;

        var parts = dependency.Split('-');
        if (parts.Length < 3)
            return dependency.Trim();

        var versionCandidate = parts[^1];
        if (!char.IsDigit(versionCandidate.FirstOrDefault()))
            return dependency.Trim();

        var nameParts = parts.Take(parts.Length - 1);
        return string.Join('-', nameParts).Trim();
    }

    private static Version ParseVersion(string version)
    {
        if (Version.TryParse(version, out var parsed))
            return parsed;

        var cleaned = new string(version.TakeWhile(c => char.IsDigit(c) || c == '.').ToArray());
        if (Version.TryParse(cleaned, out parsed))
            return parsed;

        return new Version(0, 0, 0);
    }

    public static string BuildRawXml(IEnumerable<Manifest> manifests)
    {
        var settings = new XmlWriterSettings
        {
            Indent = true,
            OmitXmlDeclaration = false
        };

        using var sw = new StringWriter();
        using var writer = XmlWriter.Create(sw, settings);

        writer.WriteStartDocument();
        writer.WriteStartElement("ModLinks", SerializationConstants.NAMESPACE);

        foreach (var manifest in manifests)
        {
            writer.WriteStartElement("Manifest");

            writer.WriteElementString("Version", manifest.Version.Value.ToString());
            writer.WriteElementString("Name", manifest.Name ?? string.Empty);
            writer.WriteElementString("Repository", manifest.Repository ?? string.Empty);
            writer.WriteElementString("Issues", manifest.Issues ?? string.Empty);

            var link = manifest.Links?.Windows ?? new Link { SHA256 = string.Empty, URL = string.Empty };
            writer.WriteStartElement("Link");
            writer.WriteAttributeString("SHA256", link.SHA256 ?? string.Empty);
            writer.WriteString(link.URL ?? string.Empty);
            writer.WriteEndElement();

            writer.WriteStartElement("Dependencies");
            foreach (var dep in manifest.Dependencies ?? Array.Empty<string>())
                writer.WriteElementString("Dependency", dep);
            writer.WriteEndElement();

            writer.WriteElementString("Description", manifest.Description ?? string.Empty);

            writer.WriteStartElement("Tags");
            foreach (var tag in manifest.Tags ?? Array.Empty<string>())
                writer.WriteElementString("Tag", tag);
            writer.WriteEndElement();

            writer.WriteStartElement("Integrations");
            foreach (var integration in manifest.Integrations ?? Array.Empty<string>())
                writer.WriteElementString("Integration", integration);
            writer.WriteEndElement();

            writer.WriteStartElement("Authors");
            foreach (var author in manifest.Authors ?? Array.Empty<string>())
                writer.WriteElementString("Author", author);
            writer.WriteEndElement();

            writer.WriteEndElement();
        }

        writer.WriteEndElement();
        writer.WriteEndDocument();

        writer.Flush();
        return sw.ToString();
    }
}
