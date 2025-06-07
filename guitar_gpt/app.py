import os
from flask import Flask, request, render_template, jsonify
import openai

app = Flask(__name__)

# Ensure the OpenAI API key is set via environment variable
openai.api_key = os.getenv("OPENAI_API_KEY")
if not openai.api_key:
    raise RuntimeError("OPENAI_API_KEY environment variable not set")

SYSTEM_PROMPT = (
    "You are an experienced electric guitar instructor. "
    "Provide tips to help students improve skills such as solo speed, melody creation, and technique."
)

@app.route("/", methods=["GET"])
def index():
    return render_template("index.html")

@app.route("/chat", methods=["POST"])
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
