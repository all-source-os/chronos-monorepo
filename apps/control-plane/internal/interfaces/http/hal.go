package http

import "os"

// Link represents a HAL hypermedia link.
type Link struct {
	Href      string `json:"href"`
	Title     string `json:"title,omitempty"`
	Templated bool   `json:"templated,omitempty"`
}

// HALResource is a composable struct that adds HAL _links to any response.
// Embed it in response DTOs to include standardized hypermedia links.
type HALResource struct {
	Links map[string]Link `json:"_links,omitempty"`
}

// LinkOption configures a Link.
type LinkOption func(*Link)

// WithTitle sets the title on a Link.
func WithTitle(title string) LinkOption {
	return func(l *Link) {
		l.Title = title
	}
}

// WithTemplated marks a Link as a URI template.
func WithTemplated() LinkOption {
	return func(l *Link) {
		l.Templated = true
	}
}

// NewLink creates a Link with the given href and optional configuration.
func NewLink(href string, opts ...LinkOption) Link {
	l := Link{Href: href}
	for _, opt := range opts {
		opt(&l)
	}
	return l
}

// SelfLink creates a "self" Link for the given path.
func SelfLink(path string) Link {
	return Link{Href: path}
}

// MergeLinks combines multiple link maps into one. Later maps override earlier ones for duplicate keys.
func MergeLinks(maps ...map[string]Link) map[string]Link {
	result := make(map[string]Link)
	for _, m := range maps {
		for k, v := range m {
			result[k] = v
		}
	}
	return result
}

// DataPlaneURL returns the configured DATA_PLANE_URL for cross-service links.
// Returns empty string if not set.
func DataPlaneURL() string {
	return os.Getenv("DATA_PLANE_URL")
}

// DataPlaneLink creates a templated link pointing to the Query Service (data plane).
func DataPlaneLink(pathTemplate string, opts ...LinkOption) Link {
	base := DataPlaneURL()
	opts = append([]LinkOption{WithTemplated()}, opts...)
	return NewLink(base+pathTemplate, opts...)
}
