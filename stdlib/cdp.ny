// stdlib/cdp.ny - Customer Data Platform utilities for Hyper-Personalization

// Expands the data model to capture a wider range of customer interactions
// and data points, enabling a comprehensive 360-degree customer view.

// Represents a single customer with core data points.
type Customer struct {
    id: str,
    name: str,
    email: str,
    created_at: i64, // Unix timestamp
    updated_at: i64, // Unix timestamp
}

// Defines the various types of customer interactions that can be tracked.
type InteractionType enum {
    WebsiteVisit,
    PageView,
    EmailOpen,
    EmailClick,
    SocialMediaLike,
    SocialMediaShare,
    Purchase,
    FormSubmission,
}

// Represents a single customer interaction or data point.
// This is the core data structure for building the 360-degree customer view.
type Interaction struct {
    customerId: str,
    interactionType: InteractionType,
    timestamp: i64, // Unix timestamp
    source: str, // e.g., "website", "mobile-app", "social-media"
    details: str, // JSON string for additional data, e.g., {"url": "/pricing", "value": 99.99}
}


// --- Customer Management Functions ---

// Creates a new customer profile.
fn new_customer(id: str, name: str, email: str, timestamp: i64) -> Customer {
    c :~ Customer;
    c.id = id;
    c.name = name;
    c.email = email;
    c.created_at = timestamp;
    c.updated_at = timestamp;
    return c;
}

// Updates a customer's email and modification timestamp.
fn update_customer_email(c: &Customer, new_email: str, timestamp: i64) {
    c.email = new_email;
    c.updated_at = timestamp;
}


// --- Interaction Tracking Functions ---

// Tracks a new customer interaction event.
fn track_interaction(
    customerId: str,
    interactionType: InteractionType,
    timestamp: i64,
    source: str,
    details: str
) -> Interaction {
    i :~ Interaction;
    i.customerId = customerId;
    i.interactionType = interactionType;
    i.timestamp = timestamp;
    i.source = source;
    i.details = details;
    return i;
}


// --- Hyper-Personalization Logic (Illustrative Example) ---

// This is a simplified example to illustrate how the expanded data model can be used.
// In a real-world scenario, this would involve more complex data analysis,
// segmentation, and possibly machine learning models.

// Determines if a customer is eligible for a special offer based on their
// recent activity. This enables hyper-personalized marketing campaigns.
fn should_receive_special_offer(interactions: &Interaction[], customerId: str, now: i64) -> bool {
    purchase_count :~ i32 = 0;
    has_recent_visit :~ bool = false;

    i :~ i64 = 0;
    while i < interactions.len() {
        interaction := interactions[i];
        if interaction.customerId == customerId {
            // Check for purchases
            if interaction.interactionType == InteractionType::Purchase {
                purchase_count += 1;
            }
            // Check for recent website visits (e.g., in the last 7 days)
            if interaction.interactionType == InteractionType::WebsiteVisit {
                // 604800 seconds = 7 days
                if (now - interaction.timestamp) < 604800 {
                    has_recent_visit = true;
                }
            }
        }
        i += 1;
    }

    // Example personalization rule:
    // Offer to frequent buyers (more than 5 purchases) who have visited the site recently.
    if purchase_count > 5 && has_recent_visit {
        return true;
    }

    return false;
}
