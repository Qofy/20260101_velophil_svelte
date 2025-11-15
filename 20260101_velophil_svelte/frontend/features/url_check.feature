Feature: URL validation and existence checking

  Scenario: Valid URL points to a file
    Given I enter the URL "https://example.com"
    When I wait for the check to complete
    Then I should see "✅ https://example.com exists and points to a file."

  Scenario: Typing invalid then valid URL
    Given I enter the URL "not-a-url"
    Then I should see "⚠️ Invalid URL format."
    When I enter the URL "https://example.com"
    And I wait for the check to complete
    Then I should see "✅ https://example.com exists and points to a file."

  Scenario: URL does not exist
    Given I enter the URL "https://invalid-url.com"
    When I wait for the check to complete
    Then I should see "❌ https://invalid-url.com does not exist."

  Scenario: Typing a new URL clears the old result immediately
    Given I enter the URL "https://first.com"
    And I wait 1 second
    And I enter the URL "https://typing.com"
    Then I should immediately see an empty status
    And I wait 1.5 seconds
    Then I should see "https://typing.com"

  Scenario: Old result is ignored if input changed
    Given I enter the URL "https://first.com"
    And I wait 500 milliseconds
    And I enter the URL "https://second.com"
    And I wait 1.5 seconds
    Then I should see "https://second.com"
    And I should not see "https://first.com"

  Scenario: Invalid URL should not trigger server check
    Given I enter the URL "invalid-url"
    And I wait 2 seconds
    Then I should see "⚠️ Invalid URL format."

  Scenario: Spinner appears before result
    Given I enter the URL "https://checking.com"
    And I wait 1 second
    Then I should see "⏳ Checking existence..."

