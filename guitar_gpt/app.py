import os
from functools import wraps
from flask import Flask, request, render_template, jsonify, session, redirect, url_for
import openai

app = Flask(__name__)
app.secret_key = os.getenv("FLASK_SECRET_KEY", "dev-secret-change-me")

# Ensure the OpenAI API key is set via environment variable
openai.api_key = os.getenv("OPENAI_API_KEY")
if not openai.api_key:
    raise RuntimeError("OPENAI_API_KEY environment variable not set")

SYSTEM_PROMPT = (
    "You are an experienced electric guitar instructor. "
    "Provide tips to help students improve skills such as solo speed, melody creation, and technique."
)

APP_USERNAME = os.getenv("APP_USERNAME", "admin")
APP_PASSWORD = os.getenv("APP_PASSWORD", "password")


def login_required(view):
    @wraps(view)
    def wrapped(*args, **kwargs):
        if not session.get("user"):
            return redirect(url_for("login"))
        return view(*args, **kwargs)
    return wrapped


@app.route("/login", methods=["GET", "POST"])
def login():
    error = None
    if request.method == "POST":
        username = request.form.get("username", "")
        password = request.form.get("password", "")
        if username == APP_USERNAME and password == APP_PASSWORD:
            session["user"] = username
            return redirect(url_for("index"))
        error = "Invalid username or password."
    return render_template("login.html", error=error)


@app.route("/logout", methods=["POST"])
def logout():
    session.pop("user", None)
    return redirect(url_for("login"))


@app.route("/", methods=["GET"])
@login_required
def index():
    return render_template("index.html")


@app.route("/chat", methods=["POST"])
@login_required
def chat():
    user_prompt = request.form.get("prompt")
    if not user_prompt:
        return jsonify({"error": "No prompt provided"}), 400

    messages = [
        {"role": "system", "content": SYSTEM_PROMPT},
        {"role": "user", "content": user_prompt},
    ]
    response = openai.ChatCompletion.create(model="gpt-3.5-turbo", messages=messages)
    answer = response["choices"][0]["message"]["content"].strip()
    return jsonify({"answer": answer})

if __name__ == "__main__":
    app.run(host="0.0.0.0", port=5000, debug=True)
