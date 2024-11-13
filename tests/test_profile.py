from goose.profile import ToolkitSpec, ObserverSpec


def test_profile_info(profile_factory):
    profile = profile_factory(
        {
            "provider": "provider",
            "processor": "processor",
            "toolkits": [ToolkitSpec("developer"), ToolkitSpec("github")],
            "observers": [ObserverSpec(name="test.plugin")],
        }
    )
    assert (
        profile.profile_info()
        == "provider:provider, processor:processor toolkits: developer, github observers: test.plugin"
    )
