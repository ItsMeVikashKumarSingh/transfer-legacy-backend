-- Migration: Seed initial CMS content structures
BEGIN;

INSERT INTO app.content (slug, body, version) VALUES 
('hero', '{"title": "Welcome to Transfer Legacy", "subtitle": "Secure your digital legacy today."}', 1),
('faqs', '{"items": [{"q": "Example Question?", "a": "Example Answer."}]}', 1),
('features', '{"items": [{"title": "Secure Vault", "description": "End-to-end encrypted storage."}]}', 1),
('team', '{"items": [{"name": "John Doe", "role": "Founder", "bio": "Passionate about digital legacy.", "imageUrl": ""}]}', 1)
ON CONFLICT (slug) DO NOTHING;

COMMIT;
