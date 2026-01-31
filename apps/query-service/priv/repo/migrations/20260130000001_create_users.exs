defmodule QueryServiceEx.Repo.Migrations.CreateUsers do
  use Ecto.Migration

  def change do
    create table(:users, primary_key: false) do
      add :id, :binary_id, primary_key: true
      add :email, :string, null: false
      add :name, :string
      add :avatar_url, :string
      add :google_id, :string, null: false
      add :google_token, :text
      add :google_refresh_token, :text

      timestamps(type: :utc_datetime)
    end

    create unique_index(:users, [:email])
    create unique_index(:users, [:google_id])
  end
end
